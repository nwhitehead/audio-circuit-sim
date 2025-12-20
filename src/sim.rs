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
    fn new_voltage_with_name(name: &str) -> Self {
        Self {
            info_type: InfoType::VOLTAGE,
            scale: 1.0,
            name: name.into(),
        }
    }
    fn new_voltage_with_name_and_scale(name: &str, scale: f64) -> Self {
        Self {
            info_type: InfoType::VOLTAGE,
            scale,
            name: name.into(),
        }
    }
    fn new_current(name: &str) -> Self {
        Self {
            info_type: InfoType::CURRENT,
            scale: 1.0,
            name: name.into(),
        }
    }
    fn new_current_with_scale(name: &str, scale: f64) -> Self {
        Self {
            info_type: InfoType::CURRENT,
            scale,
            name: name.into(),
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
        self.nodes.clear();
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
        self.set_size(self.net_size);
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

    /// Add dynamic variable to cell
    fn add_dynamic_b(&mut self, r: usize, index: usize, text: &str) {
        self.b[r].g_dyn.push(index);
        self.b[r].txt = String::from(text);
    }
    fn add_dynamic_a(&mut self, r: usize, c: usize, index: usize, text: &str) {
        self.a_matrix[r][c].g_dyn.push(index);
        self.a_matrix[r][c].txt = String::from(text);
    }
}

trait Component {
    // stamp constants into the matrix
    fn stamp(&self, m: &mut MNASystem) {}

    // update dynamic variables in m
    fn update_dynamic(&self, m: &mut MNASystem) {}

    // update state variables, only tagged nodes
    // this is intended for fixed-time compatible
    // testing to make sure we can code-gen stuff
    fn update(&mut self, m: &mut MNASystem) {
        self.update_dynamic(m);
    }

    // return true if we're done - will keep iterating
    // until all the components are happy
    fn newton(&mut self, m: &mut MNASystem) -> bool {
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
        Self {
            c,
            l0,
            l1,
            l2,
            state_var: 0.,
            voltage: 0.,
            dyn_index,
        }
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

        m.add_dynamic_b(l2, dyn_index, &format!("q:C:{},{}", l0, l1));

        // this isn't quite right as state stores 2*c*v - i/t
        // however, we'll fix this in updateFull() for display
        m.nodes[l2] =
            MNANodeInfo::new_voltage_with_name_and_scale(&format!("v:C:{},{}", l0, l1), 1.0 / c);
        self.update_dynamic(m);
    }

    fn update_dynamic(&self, m: &mut MNASystem) {
        m.set_dynamic(self.dyn_index, self.state_var);
    }

    fn update(&mut self, m: &mut MNASystem) {
        self.state_var = m.b[self.l2].lu;

        // solve legit voltage from the pins
        self.voltage = m.b[self.l0].lu - m.b[self.l1].lu;

        // then we can store this for display here
        // since this value won't be used at this point
        m.b[self.l2].lu = self.c * self.voltage;

        // Update dynamic variable since we changed state_var
        self.update_dynamic(m);
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
        self.update_dynamic(m);
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

        m.nodes[l2] = MNANodeInfo::new_current(&format!("i:V({:.}:{},{})", v, l0, l1));
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
        m.nodes[l2] = MNANodeInfo::new_voltage_with_name(&"v:probe");
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
        Self {
            v,
            f,
            dyn_index,
            l0,
            l1,
            l2,
        }
    }
}

impl Component for VoltageFunction {
    fn stamp(&self, m: &mut MNASystem) {
        let (v, f, dyn_index, l0, l1, l2) =
            (self.v, self.f, self.dyn_index, self.l0, self.l1, self.l2);

        // this is identical to voltage source
        // except voltage is dynanic
        m.stamp_static(-1., l0, l2, &"-1");
        m.stamp_static(1., l1, l2, &"+1");
        m.stamp_static(1., l2, l0, &"+1");
        m.stamp_static(-1., l2, l1, &"-1");

        m.add_dynamic_b(l2, dyn_index, &format!("Vfn:{},{}", l0, l1));

        m.nodes[l2] = MNANodeInfo::new_current(&format!("i:Vfn:{},{}", l0, l1));
        self.update_dynamic(m);
    }

    fn update_dynamic(&self, m: &mut MNASystem) {
        m.set_dynamic(self.dyn_index, self.v);
    }
    fn update(&mut self, m: &mut MNASystem) {
        self.v = (self.f)(m.time);
        // Update dynamic variable since we changed state_var
        self.update_dynamic(m);
    }
}

#[derive(Debug)]
struct JunctionPN {
    // variables
    geq: f64,
    ieq: f64,
    veq: f64,
    // parameters
    is: f64,
    nvt: f64,
    rnvt: f64,
    vcrit: f64,
}

impl JunctionPN {
    fn new(is: f64, n: f64) -> Self {
        let nvt = n * V_THERMAL;
        // initial state is linearized at v=0
        Self {
            geq: is / (n * V_THERMAL) + G_MIN,
            ieq: 0.0,
            veq: 0.0,
            is,
            nvt,
            rnvt: 1. / nvt,
            vcrit: nvt * f64::ln(nvt / (is * f64::sqrt(2.0))),
        }
    }

    fn linearize(&mut self, v: f64) {
        // linearize junction at the specified voltage
        //
        // ideally we could handle series resistance here as well
        // to avoid putting it on a separate node, but not sure how
        // to make that work as it looks like we'd need Lambert-W then
        let e = self.is * f64::exp(v * self.rnvt);
        let i = e - self.is + G_MIN * v;
        let g = e * self.rnvt + G_MIN;

        self.geq = g;
        self.ieq = v * g - i;
        self.veq = v;
    }

    // returns true if junction is good enough
    fn newton(&mut self, v: f64) -> bool {
        let dv = v - self.veq;
        if f64::abs(dv) < V_TOLERANCE {
            return true;
        }
        // check critical voltage and adjust voltage if over
        let vv = if v > self.vcrit {
            // this formula comes from Qucs documentation
            // https://qucs.sourceforge.net/tech/node16.html#SECTION00431000000000000000
            self.veq + self.nvt * f64::ln(f64::max(self.is, 1.0 + dv * self.rnvt))
        } else {
            v
        };
        self.linearize(vv);
        return false;
    }
}

#[derive(Debug)]
struct DiodeParameters {
    // Series resistor in model
    rs: f64,
    // Reverse bias saturation current
    is: f64,
    // Ideality factor
    n: f64,
}

impl Default for DiodeParameters {
    fn default() -> Self {
        // Default diode approximates 1N4148
        Self {
            rs: 10.0,
            is: 35.0e-12,
            n: 1.24,
        }
    }
}

#[derive(Debug)]
struct Diode {
    l0: usize,
    l1: usize,
    l2: usize,
    l3: usize,
    dyn_index0: usize,
    dyn_index1: usize,
    pn: JunctionPN,
    rs: f64,
}

impl Diode {
    fn new(m: &mut MNASystem, l0: usize, l1: usize, params: DiodeParameters) -> Self {
        let l2 = m.reserve();
        let l3 = m.reserve();
        let dyn_index0 = m.reserve_dynamic();
        let dyn_index1 = m.reserve_dynamic();
        let pn = JunctionPN::new(params.is, params.n);
        Self {
            l0,
            l1,
            l2,
            l3,
            dyn_index0,
            dyn_index1,
            rs: params.rs,
            pn,
        }
    }
}

impl Component for Diode {
    fn stamp(&self, m: &mut MNASystem) {
        let (l0, l1, l2, l3) = (self.l0, self.l1, self.l2, self.l3);

        // Diode could be built with 3 extra nodes:
        //
        // |  .  .    .       . +1 | V+
        // |  .  .    .       . -1 | V-
        // |  .  .  grs    -grs -1 | v:D
        // |  .  . -grs grs+geq  . | v:pn = ieq
        // | -1 +1   +1       .  . | i:pn
        //
        // Here grs is the 1/rs series conductance.
        //
        // This gives us the junction voltage (v:pn) and
        // current (i:pn) and the composite voltage (v:D).
        //
        // The i:pn row is an ideal transformer connecting
        // the floating diode to the ground-referenced v:D
        // where we connect the series resistance to v:pn
        // that solves the diode equation with Newton.
        //
        // We can then add the 3rd row to the bottom 2 with
        // multipliers 1 and -rs = -1/grs and drop it:
        //
        // |  .  .   . +1 | V+
        // |  .  .   . -1 | V-
        // |  .  . geq -1 | v:pn = ieq
        // | -1 +1  +1 rs | i:pn
        //
        // Note that only the v:pn row here is non-linear.
        //
        // We could even do away without the separate row for
        // the current, which would lead to the following:
        //
        // | +grs -grs     -grs |
        // | -grs +grs     +grs |
        // | -grs +grs +grs+geq | = ieq
        //
        // In practice we keep the current row since it's
        // nice to have it as an output anyway.
        //
        m.stamp_static(-1.0, l3, l0, "-1");
        m.stamp_static(1.0, l3, l1, "+1");
        m.stamp_static(1.0, l3, l2, "+1");
        m.stamp_static(1.0, l0, l3, "+1");
        m.stamp_static(-1.0, l1, l3, "-1");
        m.stamp_static(-1.0, l2, l3, "-1");
        m.stamp_static(self.rs, l3, l3, "rs:pn");
        m.add_dynamic_a(l2, l2, self.dyn_index0, &format!("gm:D"));
        m.add_dynamic_b(l2, self.dyn_index1, &format!("i0:D:{},{}", l0, l1));
        m.nodes[l2] = MNANodeInfo::new_voltage_with_name(&format!("v:D:{},{}", l0, l1));
        m.nodes[l3] = MNANodeInfo::new_current(&format!("i:D:{},{}", l0, l1));
        self.update_dynamic(m);
    }

    fn update_dynamic(&self, m: &mut MNASystem) {
        m.set_dynamic(self.dyn_index0, self.pn.geq);
        m.set_dynamic(self.dyn_index1, self.pn.ieq);
    }

    fn newton(&mut self, m: &mut MNASystem) -> bool {
        self.pn.newton(m.b[self.l2].lu)
    }
}

#[derive(Debug, PartialEq)]
enum TransistorType {
    NPN,
    PNP,
}

#[derive(Debug)]
struct BJTParameters {
    // Forward beta
    bf: f64,
    // Reverse beta
    br: f64,
    // Base resistor in model
    rb: f64,
    // Emitter resistor in model
    re: f64,
    // Collector resistor in model
    rc: f64,
    // Reverse bias saturation current
    is: f64,
    // Ideality factor
    n: f64,
    transistor_type: TransistorType,
}

// Computed parameters from other params
impl BJTParameters {
    // Forward alpha
    fn af(&self) -> f64 {
        self.bf / (1.0 + self.bf)
    }
    // Reverse alpha
    fn ar(&self) -> f64 {
        self.br / (1.0 + self.br)
    }
    // Series resistance from base-collector
    fn rsbc(&self) -> f64 {
        self.rb + self.rc
    }
    // Series resistance from base-emitter
    fn rsbe(&self) -> f64 {
        self.rb + self.re
    }
}

impl Default for BJTParameters {
    fn default() -> Self {
        // Default transistor approximates 2N3904
        Self {
            bf: 200.0,
            br: 20.0,
            rb: 5.8376,
            re: 2.65711,
            rc: 0.0001,
            is: 6.734e-15,
            n: 1.24,
            transistor_type: TransistorType::NPN,
        }
    }
}

#[derive(Debug)]
struct BJT {
    pin: [usize; 3],
    l: [usize; 4],
    dyn_pnc_ieq: usize,
    dyn_pnc_geq: usize,
    dyn_pne_ieq: usize,
    dyn_pne_geq: usize,
    pnc: JunctionPN,
    pne: JunctionPN,
    params: BJTParameters,
}

impl BJT {
    fn new(m: &mut MNASystem, b: usize, c: usize, e: usize, params: BJTParameters) -> Self {
        let pne = JunctionPN::new(params.is / params.af(), params.n);
        let pnc = JunctionPN::new(params.is / params.ar(), params.n);
        Self {
            pin: [b, c, e],
            l: [m.reserve(), m.reserve(), m.reserve(), m.reserve()],
            dyn_pnc_ieq: m.reserve_dynamic(),
            dyn_pnc_geq: m.reserve_dynamic(),
            dyn_pne_ieq: m.reserve_dynamic(),
            dyn_pne_geq: m.reserve_dynamic(),
            pnc,
            pne,
            params,
        }
    }
}

impl Component for BJT {
    fn stamp(&self, m: &mut MNASystem) {
        // The basic idea here is the same as with diodes
        // except we do it once for each junction.
        //
        // With the transfer currents sourced from the
        // diode currents, NPN then looks like this:
        //
        // 0 |  .  .  .  .  . 1-ar 1-af | vB
        // 1 |  .  .  .  .  .   -1  +af | vC
        // 2 |  .  .  .  .  .  +ar   -1 | vE
        // 3 |  .  .  . gc  .   -1    . | v:Qbc  = ic
        // 4 |  .  .  .  . ge    .   -1 | v:Qbe  = ie
        // 5 | -1 +1  . +1  . rsbc    . | i:Qbc
        // 6 | -1  . +1  . +1    . rsbe | i:Qbe
        //     ------------------------
        //      0  1  2  3  4    5    6
        //
        // For PNP version, we simply flip the junctions
        // by changing signs of (3,5),(5,3) and (4,6),(6,4).
        //
        // Also just like diodes, we have junction series
        // resistances, rather than terminal resistances.
        //
        // This works just as well, but should be kept
        // in mind when fitting particular transistors.
        //
        // Cheat sheet:
        // nets[0] pin[0]
        // nets[1] pin[1]
        // nets[2] pin[2]
        // nets[3] l[0]
        // nets[4] l[1]
        // nets[5] l[2]
        // nets[6] l[3]

        let pnp = self.params.transistor_type == TransistorType::PNP;
        // diode currents to external base
        m.stamp_static(1.0 - self.params.ar(), self.pin[0], self.l[2], "1-ar");
        m.stamp_static(1.0 - self.params.af(), self.pin[0], self.l[3], "1-ar");
        // diode currents to external collector and emitter
        m.stamp_static(-1.0, self.pin[1], self.l[2], "-1");
        m.stamp_static(-1.0, self.pin[2], self.l[3], "-1");
        // series resistances
        m.stamp_static(self.params.rsbc(), self.l[2], self.l[2], "rsbc");
        m.stamp_static(self.params.rsbe(), self.l[3], self.l[3], "rsbe");
        // current - junction connections
        // for the PNP case we flip the signs of these
        // to flip the diode junctions wrt. the above
        if pnp {
            m.stamp_static(-1.0, self.l[2], self.l[0], "-1");
            m.stamp_static(1.0, self.l[0], self.l[2], "+1");
            m.stamp_static(-1.0, self.l[3], self.l[1], "-1");
            m.stamp_static(1.0, self.l[1], self.l[3], "+1");
        } else {
            m.stamp_static(1.0, self.l[2], self.l[0], "+1");
            m.stamp_static(-1.0, self.l[0], self.l[2], "-1");
            m.stamp_static(1.0, self.l[3], self.l[1], "+1");
            m.stamp_static(-1.0, self.l[1], self.l[3], "-1");
        }
        // external voltages to collector current
        m.stamp_static(-1.0, self.l[2], self.pin[0], "-1");
        m.stamp_static(1.0, self.l[2], self.pin[1], "+1");
        // external voltages to emitter current
        m.stamp_static(-1.0, self.l[3], self.pin[0], "-1");
        m.stamp_static(1.0, self.l[3], self.pin[2], "+1");
        // source transfer currents to external pins
        m.stamp_static(self.params.ar(), self.pin[2], self.l[2], "+ar");
        m.stamp_static(self.params.af(), self.pin[1], self.l[3], "+af");
        // dynamic variables
        m.add_dynamic_a(self.l[0], self.l[0], self.dyn_pnc_geq, &format!("gm:Qbc"));
        m.add_dynamic_b(
            self.l[0],
            self.dyn_pnc_ieq,
            &format!("i0:Q:{},{},{}:cb", self.pin[0], self.pin[1], self.pin[2]),
        );
        m.add_dynamic_a(self.l[1], self.l[1], self.dyn_pne_geq, &format!("gm:Qbe"));
        m.add_dynamic_b(
            self.l[1],
            self.dyn_pne_ieq,
            &format!("i0:Q:{},{},{}:eb", self.pin[0], self.pin[1], self.pin[2]),
        );
        // voltage and current infos
        m.nodes[self.l[0]] = MNANodeInfo::new_voltage_with_name(&format!(
            "v:Q:{},{},{}:{}",
            self.pin[0],
            self.pin[1],
            self.pin[2],
            if pnp { "cb" } else { "bc" }
        ));
        m.nodes[self.l[1]] = MNANodeInfo::new_voltage_with_name(&format!(
            "v:Q:{},{},{}:{}",
            self.pin[0],
            self.pin[1],
            self.pin[2],
            if pnp { "eb" } else { "be" }
        ));
        m.nodes[self.l[2]] = MNANodeInfo::new_current_with_scale(
            &format!("i:Q:{},{},{}:bc", self.pin[0], self.pin[1], self.pin[2],),
            1.0 - self.params.ar(),
        );
        m.nodes[self.l[3]] = MNANodeInfo::new_current_with_scale(
            &format!("i:Q:{},{},{}:be", self.pin[0], self.pin[1], self.pin[2],),
            1.0 - self.params.af(),
        );
        self.update_dynamic(m);
    }

    fn update_dynamic(&self, m: &mut MNASystem) {
        m.set_dynamic(self.dyn_pnc_ieq, self.pnc.ieq);
        m.set_dynamic(self.dyn_pnc_geq, self.pnc.geq);
        m.set_dynamic(self.dyn_pne_ieq, self.pne.ieq);
        m.set_dynamic(self.dyn_pne_geq, self.pne.geq);
    }

    fn newton(&mut self, m: &mut MNASystem) -> bool {
        self.pnc.newton(m.b[self.l[0]].lu) && self.pne.newton(m.b[self.l[1]].lu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;

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
    fn test_pn() -> Result<(), String> {
        // Similar to 1N4148 (but just PN junction)
        let mut pn = JunctionPN::new(/*is=*/ 35.0e-12, /*n=*/ 1.24);
        // vcrit is point where current increases faster than voltage as voltage increases
        assert!(approx_eq!(f64, pn.vcrit, 0.6542963597947701, ulps = 100));
        // Check ieq for a couple voltages
        pn.newton(0.5);
        assert!(approx_eq!(f64, pn.ieq, 0.002760783529589722, ulps = 100));
        pn.newton(0.4);
        assert!(approx_eq!(f64, pn.ieq, 0.0000976127760265226, ulps = 100));
        // 0.4 should just take 1 newton step (below vcrit)
        let mut done = pn.newton(0.4);
        assert!(done);
        // 0.8 takes more than 2 iterations because of qucs current thing
        done = pn.newton(0.8);
        assert!(!done);
        done = pn.newton(0.8);
        assert!(!done);
        // But with more iterations it should converge
        for i in 0..10 {
            done = pn.newton(0.8);
            if done {
                break;
            }
        }
        assert!(done);
        Ok(())
    }

    #[test]
    fn test_component_polymorphism() -> Result<(), String> {
        let mut s = MNASystem::default();
        s.set_size(3);
        let c1 = Resistor::new(&mut s, 100.0, 0, 1);
        let c2 = Resistor::new(&mut s, 100.0, 1, 2);
        let c3 = Capacitor::new(&mut s, 0.1, 1, 2);
        let c4 = Diode::new(&mut s, 0, 1, DiodeParameters::default());
        println!("{:?}", &c1);
        println!("{:?}", &c3);
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
