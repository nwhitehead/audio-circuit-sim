
from dataclasses import dataclass
from enum import Enum
import numpy as np

class ComponentType(Enum):
    PASSIVE = 1
    SOURCE = 2
    NONLINEAR = 3

@dataclass
class Resistor:
    v: float
    def component_type(self):
        return ComponentType.PASSIVE
    def conductance(self):
        return 1 / self.v

@dataclass
class VoltageSupply:
    v: float
    def component_type(self):
        return ComponentType.SOURCE

def count_num_voltages(netlist):
    n = 0
    for (component, pos, neg) in netlist:
        n = max(n, pos, neg)
    return n

def count_num_sources(netlist):
    m = 0
    for (component, pos, neg) in netlist:
        if component.component_type() == ComponentType.SOURCE:
            m += 1
    return m

def generate_mna_g(netlist):
    nv = count_num_voltages(netlist)
    g = np.zeros((nv, nv), dtype=float)
    for (component, pos, neg) in netlist:
        if component.component_type() == ComponentType.PASSIVE:
            c = component.conductance()
            # On diagonal is sum of conductances
            if pos > 0:
                g[pos - 1, pos - 1] += c
            if neg > 0:
                g[neg - 1, neg - 1] += c
            # Off diagonal is negation, symmetric
            if pos > 0 and neg > 0:
                g[pos - 1, neg - 1] -= c
                g[neg - 1, pos -1 ] -= c
    return g

def same_matrix(a, b):
    return np.linalg.norm(a - b) < 1e-6

def main():
    print('Python circuit simulator')
    # Example is Case 1 from:
    #     https://lpsa.swarthmore.edu/Systems/Electrical/mna/MNA3.html
    r1 = Resistor(2)
    r2 = Resistor(4)
    r3 = Resistor(8)
    vss = VoltageSupply(32)
    vextra = VoltageSupply(20)
    assert r1.component_type() == ComponentType.PASSIVE
    assert r1.conductance() == 0.5
    # Netlist is component, positive, negative connection
    # Connection 0 is always ground
    netlist = [
        (r1, 0, 1),
        (r2, 2, 3),
        (r3, 0, 2),
        (vss, 2, 1),
        (vextra, 3, 0),
    ]
    nv = count_num_voltages(netlist)
    assert nv == 3
    ns = count_num_sources(netlist)
    assert ns == 2
    g = generate_mna_g(netlist)
    assert same_matrix(g, np.array([[0.5, 0.0, 0.0], [0.0, 0.375, -0.25], [0.0, -0.25, 0.25]]))
    # Case 2
    netlist = [
        (r1, 0, 1),
        (r2, 1, 2),
        (r3, 0, 2),
        (vss, 1, 2),
    ]
    g = generate_mna_g(netlist)
    assert same_matrix(g, np.array([[0.75, -0.25], [-0.25, 0.375]]))

if __name__ == '__main__':
    main()
