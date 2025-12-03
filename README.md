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

## Modified Nodal Analysis

Basics:
https://lpsa.swarthmore.edu/Systems/Electrical/mna/MNA2.html

Idea is to use Kirchoff's Current Law at every point in circuit. Make a matrix.
Each component has N pins and M extra state variables. The pins each have a
voltage, the basic matrix is `Mx=b`. The `x` part is mostly voltages,
multiplying by `M` should give currents on right side. Generally `b` part will
be 0 but there might be some current sources that make it not 0.

For the extra state variables, those get tacked onto the matrix, variables, and
right hand side. For voltage sources the extra variable is actual current. So we
solve for current in that part (`x` will hold current), voltage will be
constant. RHS `b` for that state variable will be constant voltage.

Each component has to "stamp" what it does onto the matrices. For resistors, the
idea is to stamp conductance `1/R` positive and negative in right places based
on connections.

In `M` matrix, row `i` is related to currents summing at point `i`.

## Symbolic stuff

Want to generate code that does simulation of each circuit. Can precompute
symbolic expressions for the circuit, compile into a separate module for
efficiency. Or 'interpret' the symbolic parsed structure to simulate without
needing a compile step.

I like the idea of explicit equations, not necessarily in matrix numerical form.

OK, after thinking more about symbolic stuff I don't like doing everything with
explicit ops and then trying to organize it into a matrix. Lots of work that is
being undone, simplifying is hard problem.

