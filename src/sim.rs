/// Show pivot details in LU factorization
const VERBOSE_LU: bool = true;

/// Stepsize for linearization of non-linear components
const G_MIN: f64 = 1e-12;

/// Voltage tolerance for iterative solver
const V_TOLERANCE: f64 = 5e-5;

/// Thermal voltage for diode and transistor model
const V_THERMAL: f64 = 0.026;

/// Maximum number of iterations in main netlist loop
const MAX_ITER: u32 = 200;

//
// General overview
// ----------------
//
// Circuits are built from nodes and Components, where nodes are
// simply positive integers (with 0 designating ground).
//
// Every Component has one or more pins connecting to the circuit
// nodes as well as zero or more internal nets.
//
// While we map pins directly to nets here, the separation would
// be useful if the solver implemented stuff like net-reordering.
//
// MNACell represents a single entry in the solution matrix,
// where we store constants and time-step dependent constants
// separately, plus collect pointers to dynamic variables.
//
// We track enough information here that we only need to stamp once.
//
// Lifetime is needed to make sure dynamic references are live when used.
//
#[derive(Debug)]
struct MNACell {
    // simple values (eg. resistor conductance)
    g: f64,
    // time-scaled values (eg. capacitor conductance)
    g_timed: f64,
    // references to dynamic variables (by index into vector)
    g_dyn: Vec<usize>,
    // LU value and pre-LU cache value
    lu: f64,
    pre_lu: f64,
    // Debug info
    txt: String,
}

impl Default for MNACell {
    fn default() -> Self {
        MNACell {
            g: 0.0,
            g_timed: 0.0,
            g_dyn: vec![],
            lu: 0.0,
            pre_lu: 0.0,
            txt: String::new(),
        }
    }
}

impl MNACell {
    /// Setup pre_lu cache
    fn init_lu(&mut self, step_scale: f64) {
        self.pre_lu = self.g + self.g_timed * step_scale;
    }

    /// Restore matrix state and update dynamic values
    fn update_pre(&mut self, vars: &Vec<f64>) {
        self.lu = self.pre_lu;
        for index in self.g_dyn.iter() {
            self.lu += vars[*index];
        }
    }
}

#[derive(Debug)]
enum InfoType {
    VOLTAGE,
    CURRENT,
    COUNT,
}

// this is for keeping track of node information
// for the purposes of more intelligent plotting
#[derive(Debug)]
struct MNANodeInfo {
    // one auto-range per unit-type
    info_type: InfoType,
    // scale factor (eg. charge to voltage)
    scale: f64,
    // node name for display
    name: String,
}

impl MNANodeInfo {
    fn new_voltage(n: usize) -> Self {
        Self {
            info_type: InfoType::VOLTAGE,
            scale: 1.0,
            name: format!("v{}", n),
        }
    }
}
// Store matrix as a vector of rows for easy pivots
type MNAVector = Vec<MNACell>;
type MNAMatrix = Vec<MNAVector>;

// Stores A and b for A*x - b = 0, where x is the solution.
//
// A is stored as a vector of rows, for easy in-place pivots
//
#[derive(Debug)]
struct MNASystem {
    nodes: Vec<MNANodeInfo>,
    a_matrix: MNAMatrix,
    b: MNAVector,
    time: f64,
    net_size: usize,
    vars: Vec<f64>,
}

impl Default for MNASystem {
    fn default() -> Self {
        MNASystem {
            nodes: vec![],
            a_matrix: MNAMatrix::default(),
            b: MNAVector::default(),
            time: 0.0,
            net_size: 0,
            vars: vec![],
        }
    }
}

impl MNASystem {
    fn set_size(&mut self, n: usize) {
        self.a_matrix.resize_with(n, Default::default);
        self.b.resize_with(n, Default::default);
        for i in 0..n {
            self.a_matrix[i].resize_with(n, Default::default);
            self.nodes.push(MNANodeInfo::new_voltage(i));
        }
        self.net_size = n;
    }

    fn stamp_static(&mut self, value: f64, r: usize, c: usize, txt: &str) {
        self.a_matrix[r][c].g += value;
        self.a_matrix[r][c].txt += txt;
    }

    fn stamp_timed(&mut self, value: f64, r: usize, c: usize, txt: &str) {
        self.a_matrix[r][c].g_timed += value;
        self.a_matrix[r][c].txt += txt;
    }

    /// Reserve a fresh net position for a component's internal use
    fn reserve(&mut self) -> usize {
        let sz = self.net_size;
        self.net_size += 1;
        return sz;
    }

    /// Reserve a fresh dynamic variable for a component's state tracking
    fn reserve_dynamic(&mut self) -> usize {
        let sz = self.vars.len();
        self.vars.push(0.);
        return sz;
    }

    /// Let component update dynamic value that is referenced in cells
    fn set_dynamic(&mut self, index: usize, v: f64) {
        self.vars[index] = v;
    }
}

trait Component {
    // stamp constants into the matrix
    fn stamp(&self, m: &mut MNASystem) {}

    // update state variables, only tagged nodes
    // this is intended for fixed-time compatible
    // testing to make sure we can code-gen stuff
    fn update(&mut self, m: &mut MNASystem) {}

    // return true if we're done - will keep iterating
    // until all the components are happy
    fn newton(&self, m: &mut MNASystem) -> bool {
        true
    }

    // time-step change, fix their state-variables (used for caps)
    fn scale_time(&mut self, m: &mut MNASystem, t_old_per_new: f64) {}
}

const UNIT_VALUE_OFFSET: i32 = 4;
const UNIT_VALUE_MAX: i32 = 8;
const UNIT_VALUE_SUFFIXES: [&'static str; UNIT_VALUE_MAX as usize] =
    ["p", "n", "u", "m", "", "k", "M", "G"];

fn format_unit_value(v: f64, unit: &str) -> String {
    let mut suff: i32 = UNIT_VALUE_OFFSET + (v.log10() as i32) / 3;
    if v < 1.0 {
        suff -= 1;
    }
    if suff < 0 {
        suff = 0;
    }
    if suff > UNIT_VALUE_MAX {
        suff = UNIT_VALUE_MAX;
    }
    let vr = v / f64::powf(10.0, 3.0 * ((suff - UNIT_VALUE_OFFSET) as f64));
    // Use as many decimals as needed, or none if not needed
    return format!("{:.}{}{}", vr, UNIT_VALUE_SUFFIXES[suff as usize], unit);
}

// Components stamp themselves onto MNASystem as they are created.

#[derive(Debug)]
struct Resistor {
    r: f64,
    l0: usize,
    l1: usize,
}

impl Resistor {
    fn new(m: &mut MNASystem, r: f64, l0: usize, l1: usize) -> Self {
        Self { r, l0, l1 }
    }
}

impl Component for Resistor {
    fn stamp(&self, m: &mut MNASystem) {
        let (r, l0, l1) = (self.r, self.l0, self.l1);
        let g = 1.0 / r;
        let txt = format!("R{}", format_unit_value(r, ""));
        m.stamp_static(g, l0, l0, &format!("+{}", txt));
        m.stamp_static(-g, l0, l1, &format!("-{}", txt));
        m.stamp_static(-g, l1, l0, &format!("-{}", txt));
        m.stamp_static(g, l1, l1, &format!("+{}", txt));
    }
}

#[derive(Debug)]
struct Capacitor {
    c: f64,
    l0: usize,
    l1: usize,
    l2: usize,
    state_var: f64,
    dyn_index: usize,
    voltage: f64,
}

impl Capacitor {
    fn new(m: &mut MNASystem, c: f64, l0: usize, l1: usize) -> Self {
        let l2 = m.reserve();
        let dyn_index = m.reserve_dynamic();
        Self { c, l0, l1, l2, state_var: 0., voltage: 0., dyn_index }
    }
}

impl Component for Capacitor {
    fn stamp(&self, m: &mut MNASystem) {
        // we can use a trick here, to get the capacitor to
        // work on it's own line with direct trapezoidal:
        //
        // | -g*t  +g*t  +t | v+
        // | +g*t  -g*t  -t | v-
        // | +2*g  -2*g  -1 | state
        //
        // the logic with this is that for constant timestep:
        //
        //  i1 = g*v1 - s0   , s0 = g*v0 + i0
        //  s1 = 2*g*v1 - s0 <-> s0 = 2*g*v1 - s1
        //
        // then if we substitute back:
        //  i1 = g*v1 - (2*g*v1 - s1)
        //     = s1 - g*v1
        //
        // this way we just need to copy the new state to the
        // next timestep and there's no actual integration needed
        //
        // the "half time-step" error here means that our state
        // is 2*c*v - i/t but we fix this for display in update
        // and correct the current-part on time-step changes
        //
        // trapezoidal needs another factor of two for the g
        // since c*(v1 - v0) = (i1 + i0)/(2*t), where t = 1/T
        let (c, l0, l1, l2, dyn_index) = (self.c, self.l0, self.l1, self.l2, self.dyn_index);
        let txt = format_unit_value(c, "F");
        let g = 2.0 * c;
        m.stamp_timed(1., l0, l2, "+t");
        m.stamp_timed(-1., l1, l2, "-t");
        m.stamp_timed(-g, l0, l0, &format!("-t*{}", txt));
        m.stamp_timed(g, l0, l1, &format!("+t*{}", txt));
        m.stamp_timed(g, l1, l0, &format!("+t*{}", txt));
        m.stamp_timed(-g, l1, l1, &format!("-t*{}", txt));
        m.stamp_static(2. * g, l2, l0, &format!("+2*{}", txt));
        m.stamp_static(-2. * g, l2, l1, &format!("-2*{}", txt));
        m.stamp_static(-1., l2, l2, &"-1");

        m.b[l2].g_dyn.push(dyn_index);
        m.set_dynamic(dyn_index, self.state_var);
        m.b[l2].txt = String::from(format!("q:C:{},{}", l0, l1));
        // this isn't quite right as state stores 2*c*v - i/t
        // however, we'll fix this in updateFull() for display
        m.nodes[l2].name = String::from(format!("v:C:{},{}", l0, l1));
        m.nodes[l2].scale = 1. / c;
    }

    fn update(&mut self, m: &mut MNASystem) {
        self.state_var = m.b[self.l2].lu;

        // solve legit voltage from the pins
        self.voltage = m.b[self.l0].lu - m.b[self.l1].lu;

        // then we can store this for display here
        // since this value won't be used at this point
        m.b[self.l2].lu = self.c * self.voltage;

        // Update dynamic variable since we changed state_var
        m.set_dynamic(self.dyn_index, self.state_var);
    }

    fn scale_time(&mut self, m: &mut MNASystem, t_old_per_new: f64) {
        // the state is 2*c*voltage - i/t0
        // so we subtract out the voltage, scale current
        // and then add the voltage back to get new state
        //
        // note that this also works if the old rate is infinite
        // (ie. t0=0) when going from DC analysis to transient
        //
        let qq = 2. * self.c * self.voltage;
        self.state_var = qq + (self.state_var - qq) * t_old_per_new;

        // Update dynamic variable since we changed state_var
        m.set_dynamic(self.dyn_index, self.state_var);
    }

}

#[derive(Debug)]
struct VoltageSource {
    v: f64,
    l0: usize,
    l1: usize,
    l2: usize,
}

impl VoltageSource {
    fn new(m: &mut MNASystem, v: f64, l0: usize, l1: usize) -> Self {
        let l2 = m.reserve();
        Self { v, l0, l1, l2 }
    }
}

impl Component for VoltageSource {
    fn stamp(&self, m: &mut MNASystem) {
        let (v, l0, l1, l2) = (self.v, self.l0, self.l1, self.l2);
        m.stamp_static(-1., l0, l2, &"-1");
        m.stamp_static(1., l1, l2, &"+1");
        m.stamp_static(1., l2, l0, &"+1");
        m.stamp_static(-1., l2, l1, &"-1");

        m.b[l2].g = v;
        m.b[l2].txt = String::from(format!("{:.}V", v));

        m.nodes[l2].name = format!("i:V({:.}:{},{})", v, l0, l1);
        m.nodes[l2].info_type = InfoType::CURRENT;
    }
}

#[derive(Debug)]
struct VoltageProbe {
    // probe a differential voltage
    // also forces this voltage to actually get solved :)
    l0: usize,
    l1: usize,
    l2: usize,
}

impl VoltageProbe {
    fn new(m: &mut MNASystem, l0: usize, l1: usize) -> Self {
        let l2 = m.reserve();
        Self { l0, l1, l2 }
    }
}

impl Component for VoltageProbe {
    fn stamp(&self, m: &mut MNASystem) {
        let (l0, l1, l2) = (self.l0, self.l1, self.l2);

        // vp + vn - vd = 0     so      vp = vd - vn
        m.stamp_static(1., l2, l0, "+1");
        m.stamp_static(-1., l2, l1, "-1");
        m.stamp_static(-1., l2, l2, "-1");
        m.nodes[l2].name = "v:probe".into();
    }
}


#[derive(Debug)]
struct VoltageFunction {
    // probe a differential voltage
    // also forces this voltage to actually get solved :)
    v: f64,
    f: fn(f64) -> f64,
    dyn_index: usize,
    l0: usize,
    l1: usize,
    l2: usize,
}

impl VoltageFunction {
    fn new(m: &mut MNASystem, f: fn(f64) -> f64, l0: usize, l1: usize) -> Self {
        let l2 = m.reserve();
        let dyn_index = m.reserve_dynamic();
        let v = f(0.0);
        Self { v, f, dyn_index, l0, l1, l2 }
    }
}

impl Component for VoltageFunction {
    fn stamp(&self, m: &mut MNASystem) {
        let (v, f, dyn_index, l0, l1, l2) = (self.v, self.f, self.dyn_index, self.l0, self.l1, self.l2);

        // this is identical to voltage source
        // except voltage is dynanic
        m.stamp_static(-1., l0, l2, &"-1");
        m.stamp_static(1., l1, l2, &"+1");
        m.stamp_static(1., l2, l0, &"+1");
        m.stamp_static(-1., l2, l1, &"-1");

        m.b[l2].g_dyn.push(dyn_index);
        m.set_dynamic(dyn_index, self.v);
        m.b[l2].txt = String::from(format!("Vfn:{},{}", l0, l1));

        m.nodes[l2].name = format!("i:Vfn:{},{}", l0, l1);
        m.nodes[l2].info_type = InfoType::CURRENT;
    }

    fn update(&mut self, m: &mut MNASystem) {
        self.v = (self.f)(m.time);
        // Update dynamic variable since we changed state_var
        m.set_dynamic(self.dyn_index, self.v);
    }

}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_unit_value() -> Result<(), String> {
        assert_eq!(format_unit_value(1.5, " Ohms"), "1.5 Ohms");
        assert_eq!(format_unit_value(15.0, " Ohms"), "15 Ohms");
        assert_eq!(format_unit_value(1500.0, " Ohms"), "1.5k Ohms");
        assert_eq!(format_unit_value(150000.0, " Ohms"), "150k Ohms");
        assert_eq!(format_unit_value(1500000.0, " Ohms"), "1.5M Ohms");
        assert_eq!(format_unit_value(0.015, " Ohms"), "15m Ohms");
        assert_eq!(format_unit_value(0.0015, " Ohms"), "1.5m Ohms");
        assert_eq!(format_unit_value(0.00015, " Ohms"), "150u Ohms");
        Ok(())
    }

    #[test]
    fn test_system() -> Result<(), String> {
        let mut s = MNASystem::default();
        s.set_size(5);
        assert_eq!(s.a_matrix.len(), 5);
        for row in s.a_matrix {
            assert_eq!(row.len(), 5);
        }
        Ok(())
    }

    #[test]
    fn test_component_polymorphism() -> Result<(), String> {
        let mut s = MNASystem::default();
        s.set_size(3);
        let c1 = Resistor::new(&mut s, 100.0, 0, 1);
        let c2 = Resistor::new(&mut s, 100.0, 1, 2);
        let c3 = Capacitor::new(&mut s, 0.1, 1, 2);
        println!("{:?}", &c1);
        c1.stamp(&mut s);
        c2.stamp(&mut s);
        c3.stamp(&mut s);
        let mut v: Vec<Box<dyn Component>> = vec![Box::new(c1), Box::new(c2), Box::new(c3)];
        Ok(())
    }
}

fn main() {
    let mut s = MNASystem::default();
    s.set_size(3);
    let c1 = Resistor::new(&mut s, 100.0, 0, 1);
    let c2 = Resistor::new(&mut s, 100.0, 1, 2);
    let c3 = Capacitor::new(&mut s, 0.1, 1, 2);

    println!("Hello from sim.rs");
    println!("{:?}", s);
    println!("Resistor is {}", format_unit_value(1500.0, " Ohms"));
}
