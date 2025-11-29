# Circuit

This is a Rust project for drawing and simulating audio electronic circuits.

## Idea

For drawing: start with KiCAD `.lib` files that have drawing commands.
Parse `.lib` file with a Python script, saves as JSON. Use JSON file in rust
to do drawing. Rust code uses egui drawing library.

Working out data structures and file formats beyond that.

Simulation follows general SPICE method, with some simplifications to limit
scope to audio rate data.

Example command for converting KiCAD file to JSON:
```bash
uv run script/lib2json.py ../kicad-symbols/Transistor_BJT.lib > BJT.json
```

## Python

Python part is to prototype setting up symbolic matrix solver.

```bash
python src/sim.py
```

## Parts

* C
* D
* D_Schottky
* D_Zener
* L
* LED
* Opamp_Dual
* Q_NPN_BCE
* Q_NPN_Darlington_BCE
* Q_PNP_BCE
* Q_PNP_Darlington_BCE
* R
* R_Potentiometer
* R_US
* R_Potentiometer_US
* Voltmeter_DC
