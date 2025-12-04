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
struct MNACell<'a> {
    // simple values (eg. resistor conductance)
    g: f64,
    // time-scaled values (eg. capacitor conductance)
    g_timed: f64,
    // references to dynamic variables
    g_dyn: Vec<&'a f64>,
    // LU value and pre-LU cache value
    lu: f64,
    pre_lu: f64,
    // Debug info
    txt: String,
}

impl Default for MNACell<'_> {
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

impl<'a> MNACell<'a> {
    /// Setup pre_lu cache
    fn init_lu(&mut self, step_scale: f64) {
        self.pre_lu = self.g + self.g_timed * step_scale;
    }

    /// Restore matrix state and update dynamic values
    fn update_pre(&mut self) {
        self.lu = self.pre_lu;
        for d in self.g_dyn.iter() {
            self.lu += *d;
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
type MNAVector<'a> = Vec<MNACell<'a>>;
type MNAMatrix<'a> = Vec<MNAVector<'a>>;

// Stores A and b for A*x - b = 0, where x is the solution.
//
// A is stored as a vector of rows, for easy in-place pivots
//
#[derive(Debug)]
struct MNASystem<'a> {
    nodes: Vec<MNANodeInfo>,
    a_matrix: MNAMatrix<'a>,
    b: MNAVector<'a>,
    time: f64,
    net_size: usize,
}

impl Default for MNASystem<'_> {
    fn default() -> Self {
        MNASystem {
            nodes: vec![],
            a_matrix: MNAMatrix::default(),
            b: MNAVector::default(),
            time: 0.0,
            net_size: 0,
        }
    }
}

impl<'a> MNASystem<'a> {
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

    /// Reserve a fresh variable for a comonent's internal state tracking
    fn reserve(&mut self) -> usize {
        let sz = self.net_size;
        self.net_size += 1;
        return sz;
    }
}

trait Component {
    // update state variables, only tagged nodes
    // this is intended for fixed-time compatible
    // testing to make sure we can code-gen stuff
    fn update(&self, m: &mut MNASystem) {}

    // return true if we're done - will keep iterating
    // until all the components are happy
    fn newton(&self, m: &mut MNASystem) -> bool {
        true
    }

    // time-step change, fix their state-variables (used for caps)
    fn scale_time(&mut self, t_old_per_new: f64) {}
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
        let g = 1.0 / r;
        let txt = format!("R{}", format_unit_value(r, ""));
        m.stamp_static(g, l0, l0, &format!("+{}", txt));
        m.stamp_static(-g, l0, l1, &format!("-{}", txt));
        m.stamp_static(-g, l1, l0, &format!("-{}", txt));
        m.stamp_static(g, l1, l1, &format!("+{}", txt));
        Self { r, l0, l1 }
    }
}

impl Component for Resistor {}

#[derive(Debug)]
struct Capacitor {
    c: f64,
    l0: usize,
    l1: usize,
    l2: usize,
    state_var: f64,
    voltage: f64,
}

impl Capacitor {
    fn new(m: &mut MNASystem, c: f64, l0: usize, l1: usize) -> Self {
        let l2 = m.reserve();
        let txt = format!("{}", format_unit_value(c, "F"));
        let g = 2.0 * c;
        m.stamp_timed(1., l0, l2, "+t");
        m.stamp_timed(1., l1, l2, "-t");

        // m.stamp_static(g, l0, l0, &"+t");
        // m.stamp_static(-g, l0, l1, &format!("-{}", txt));
        // m.stamp_static(-g, l1, l0, &format!("-{}", txt));
        // m.stamp_static(g, l1, l1, &format!("+{}", txt));
        Self { c, l0, l1, l2, state_var: 0., voltage: 0. }
    }
}

impl Component for Capacitor {}


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
        println!("{:?}", &c1);
        let mut v: Vec<Box<dyn Component>> = vec![Box::new(c1), Box::new(c2)];
        Ok(())
    }
}

fn main() {
    let mut system = MNASystem::default();
    system.set_size(2);
    let c1 = Resistor::new(&mut system, 100.0, 0, 1);
    println!("Hello from sim.rs");
    println!("{:?}", system);
    println!("Resistor is {}", format_unit_value(1500.0, " Ohms"));
}
