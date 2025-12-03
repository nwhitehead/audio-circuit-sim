
# List available commands
default:
  just --list

# Build and run the Rust gui
gui:
  cargo run --bin gui

# Build and run the Rust simulation
sim:
  cargo run --bin sim

# Run the Python simulation
py:
  python src/sim.py

# Run ngspice test
ngspice:
  mkdir -p out/
  ngspice test/t3.cir

# Test the tests
test:
  pytest src/parse.py

# Test halite
halite:
  g++ src/halite.cpp -o target/halite
  ./target/halite | tee out/halite.out
