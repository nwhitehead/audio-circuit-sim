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
    /// Clear cell
    fn clear(self: &mut Self) {
        self.g = 0.0;
        self.g_timed = 0.0;
        self.txt = String::new();
    }

    /// Setup pre_lu cache
    fn init_lu(self: &mut Self, step_scale: f64) {
        self.pre_lu = self.g + self.g_timed * step_scale;
    }

    /// Restore matrix state and update dynamic values
    fn update_pre(self: &mut Self) {
        self.lu = self.pre_lu;
        for d in self.g_dyn.iter() {
            self.lu += *d;
        }
    }
}

#[derive(Debug)]
enum InfoType
{
    VOLTAGE, CURRENT, COUNT
}

// this is for keeping track of node information
// for the purposes of more intelligent plotting
#[derive(Debug)]
struct MNANodeInfo
{

    // one auto-range per unit-type
    info_type: InfoType,
    // scale factor (eg. charge to voltage)
    scale: f64,
    // node name for display
    name: String,
}

impl Default for MNANodeInfo {
    fn default() -> Self {
        MNANodeInfo {
            info_type: InfoType::VOLTAGE,
            scale: 1.0,
            name: String::new(),
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
struct MNASystem<'a>
{
    nodes: Vec<MNANodeInfo>,
    a_matrix: MNAMatrix<'a>,
    b: MNAVector<'a>,
    time: f64,
}

impl Default for MNASystem<'_> {
    fn default() -> Self {
        MNASystem {
            nodes: vec![],
            a_matrix: MNAMatrix::default(),
            b: MNAVector::default(),
            time: 0.0,
        }
    }
}


impl <'a> MNASystem<'a> {
    fn set_size(self: &mut Self, n: usize) {
        self.a_matrix.resize_with(n, Default::default);
        self.b.resize_with(n, Default::default);
        self.nodes.resize_with(n, Default::default);
        for i in 0..n {
            self.b[i].clear();
            self.a_matrix[i].resize_with(n, Default::default);
            self.nodes[i] = MNANodeInfo {
                info_type: InfoType::VOLTAGE,
                scale: 1.0,
                name: format!("v{}", i),
            };
            for j in 0..n {
                self.a_matrix[i][j].clear();
            }
        }
    }
}

fn main() {
    let mut system = MNASystem::default();
    system.set_size(3);
    println!("Hello from sim.rs");
    println!("{:?}", system);
}
