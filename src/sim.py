
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
    n = -1
    for (component, pos, neg) in netlist:
        n = max(n, pos, neg)
    return n + 1

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
            a[pos, pos] += c
            a[neg, neg] += c
            # Off diagonal is sum of negated conductances, symmetric
            a[pos, neg] -= c
            a[neg, pos] -= c
        elif t == ComponentType.VOLTAGE_SOURCE:
            # Voltage sources go into upper right and lower left sections
            # Positive 1 for positive connection side, -1 for negative side
            a[pos, n + mi] = 1
            a[n + mi, pos] = 1
            a[neg, n + mi] = -1
            a[n + mi, neg] = -1
            # Also record voltage in RHS
            b[n + mi] = component.v
            mi += 1
        elif t == ComponentType.CURRENT_SOURCE:
            # Current source point from negative to positive
            # So more like current out, current in
            b[pos] += component.i
            b[neg] -= component.i
    return (a, b)

def main():
    # Example is Case 1 from:
    #     https://lpsa.swarthmore.edu/Systems/Electrical/mna/MNA3.html
    r1 = Resistor(2)
    r2 = Resistor(4)
    r3 = Resistor(8)
    vss = VoltageSource(32)
    vextra = VoltageSource(20)
    iss = CurrentSource(0.25)
    assert r1.component_type() == ComponentType.PASSIVE
    assert r1.conductance() == 0.5

    # Netlist is component, positive, negative connection
    # Connection 0 is always ground
    netlist = [
        (r1, 1, 0),
        (r2, 2, 3),
        (r3, 2, 0),
        (vss, 2, 1),
        (vextra, 3, 0),
    ]
    assert count_num_voltages(netlist) == 4
    assert count_num_voltage_sources(netlist) == 2
    a, b = generate_mna(netlist)
    assert np.allclose(a, np.array([[0.625, -0.5, -0.125, 0, 0, -1], [-0.5, 0.5, 0, 0, -1, 0], [-0.125, 0, 0.375, -0.25, 1, 0],
                                    [0, 0, -0.25, 0.25, 0, 1], [0, -1, 1, 0, 0, 0], [-1, 0, 0, 1, 0, 0]]))
    assert np.allclose(b, np.array([0, 0, 0, 0, 32, 20]))
    x = np.linalg.solve(a, b)
    # Check relative voltages to ground
    assert np.allclose(x[1:4] - x[0], [-8, 24, 20])
    # Check currents
    assert np.allclose(x[4:], [-4, 1])

    # Case 2
    netlist = [
        (r1, 1, 0),
        (r2, 2, 1),
        (r3, 2, 0),
        (vss, 1, 2),
        (iss, 1, 0),
    ]
    a, b = generate_mna(netlist)
    assert np.allclose(a, np.array([[0.625, -0.5, -0.125, 0], [-0.5, 0.75, -0.25, 1], [-0.125, -0.25, 0.375, -1], [0, 1, -1, 0]]))
    assert np.allclose(b, np.array([-0.25, 0.25, 0, 32]))
    x = np.linalg.solve(a, b)
    # Check relative voltages to ground
    assert np.allclose(x[1:3] - x[0], [6.8, -25.2])
    # Check current
    assert np.allclose(x[3:], -11.15)


if __name__ == '__main__':
    main()
