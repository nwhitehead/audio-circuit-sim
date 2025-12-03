
pub struct Variable {
    /// Variable that we are solving for in the circuit simulation.
    name: String,
}

pub enum BinOp {
    Add, Subtract, Multiply, Divide,
}

pub enum Expression {
    V(Variable),
    Bin(BinOp, )
}

fn main() {
    println!("Hello from sim.rs");
}
