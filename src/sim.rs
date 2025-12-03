
/// Show pivot details in LU factorization
const VERBOSE_LU: bool = true;

/// Stepsize for linearization of non-linear components
const G_MIN: f64 = 1e-12;

/// Voltage tolerance for iterative solver
const V_TOLERANCE: f64 = 5e-5;

/// Thermal voltage for diode and transistor model
const V_THERMAL: f64 = 0.0258;

/// Maximum number of iterations in main netlist loop
const MAX_ITER: u32 = 200;


fn main() {
    println!("Hello from sim.rs");
}
