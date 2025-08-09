
from dataclasses import dataclass
from enum import Enum
import numpy as np

class ComponentType(Enum):
    PASSIVE = 1
    VOLTAGE_SOURCE = 2
    CURRENT_SOURCE = 3
    NONLINEAR = 4

@dataclass
class Resistor:
    r: float
    def component_type(self):
        return ComponentType.PASSIVE
    def conductance(self):
        return 1 / self.r

@dataclass
class VoltageSource:
    v: float
    def component_type(self):
        return ComponentType.VOLTAGE_SOURCE

@dataclass
class CurrentSource:
    i: float
    def component_type(self):
        return ComponentType.CURRENT_SOURCE

def count_num_voltages(netlist):
    n = 0
    for (component, pos, neg) in netlist:
        n = max(n, pos, neg)
    return n

def count_num_voltage_sources(netlist):
    m = 0
    for (component, pos, neg) in netlist:
        if component.component_type() == ComponentType.VOLTAGE_SOURCE:
            m += 1
    return m

def generate_mna(netlist):
    '''
    Given netlist, returns a matrix and b vector for a * x = b formulation
    x will have structure of n voltages followed by m currents through sources.
    
    '''
    n = count_num_voltages(netlist)
    m = count_num_voltage_sources(netlist)
    a = np.zeros((n + m, n + m), dtype=float)
    b = np.zeros(n + m, dtype=float)
    mi = 0
    for (component, pos, neg) in netlist:
        t = component.component_type()
        if t == ComponentType.PASSIVE:
            # Passive components go into upperleft n x n part of a
            c = component.conductance()
            # On diagonal is sum of conductances
            if pos > 0:
                a[pos - 1, pos - 1] += c
            if neg > 0:
                a[neg - 1, neg - 1] += c
            # Off diagonal is sum of negated conductances, symmetric
            # Off diagonal is only relevant if no ground connection
            if pos > 0 and neg > 0:
                a[pos - 1, neg - 1] -= c
                a[neg - 1, pos -1 ] -= c
        elif t == ComponentType.VOLTAGE_SOURCE:
            # Voltage sources go into upper right and lower left sections
            # Positive 1 for positive connection side, -1 for negative side
            # Connections to ground don't have an entry
            if pos > 0:
                a[pos - 1, n + mi] = 1
                a[n + mi, pos - 1] = 1
            if neg > 0:
                a[neg - 1, n + mi] = -1
                a[n + mi, neg - 1] = -1
            # Also record voltage in RHS
            b[n + mi] = component.v
            mi += 1
    return (a, b)

def same_matrix(a, b):
    return np.linalg.norm(a - b) < 1e-6

def main():
    print('Python circuit simulator')
    # Example is Case 1 from:
    #     https://lpsa.swarthmore.edu/Systems/Electrical/mna/MNA3.html
    r1 = Resistor(2)
    r2 = Resistor(4)
    r3 = Resistor(8)
    vss = VoltageSource(32)
    vextra = VoltageSource(20)
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
    assert count_num_voltages(netlist) == 3
    assert count_num_voltage_sources(netlist) == 2
    a, b = generate_mna(netlist)
    assert same_matrix(a, np.array([[0.5, 0, 0, -1, 0], [0, 0.375, -0.25, 1, 0], [0, -0.25, 0.25, 0, 1], [-1, 1, 0, 0, 0], [0, 0, 1, 0, 0]]))
    assert same_matrix(b, np.array([0, 0, 0, 32, 20]))

    # Case 2
    netlist = [
        (r1, 0, 1),
        (r2, 1, 2),
        (r3, 0, 2),
        (vss, 1, 2),
    ]
    a, b = generate_mna(netlist)
    print(a)
    print(b)
    assert same_matrix(a, np.array([[0.75, -0.25, 1], [-0.25, 0.375, -1], [1, -1, 0]]))

if __name__ == '__main__':
    main()
