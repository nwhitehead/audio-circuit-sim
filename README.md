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

## RCR File Format

The `.rcr` file format is a Rust Circuit simulatoR, a simplified SPICE format.
Lines starting with `#` are comments. Leading and trailing whitespace is
ignored. File can be ended early with `.end`. Otherwise there are component
lines:
* `Rname N+ N- value` resistor
* `Cname N+ N- value` capacitor
* `Vname N+ N- value` voltage source
* `Iname N+ N- value` current source

Nodes are named with anything including numbers. Values are numerical with
optional suffixes from `TGXKMUNPF`, any case. Meaning is tera, giga, mega, kilo,
milli, micro, nano, pico, femto.

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
