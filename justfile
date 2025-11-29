
# List available commands
default:
  just --list

# Build and run the Rust gui
gui:
  cargo run

# Run the Python simulation
py:
  python src/sim.py

# Run ngspice test
ngspice:
  mkdir -p out/
  ngspice test/t3.cir