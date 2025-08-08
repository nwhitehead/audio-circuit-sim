
from dataclasses import dataclass
from enum import Enum

class ComponentType(Enum):
    PASSIVE = 1
    VOLTAGE = 2
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
        return ComponentType.VOLTAGE

def count_num_voltages(netlist):
    n = 0
    for (component, pos, neg) in netlist:
        n = max(n, pos, neg)
    return n

def main():
    print('Python circuit simulator')
    r1 = Resistor(2)
    r2 = Resistor(4)
    r3 = Resistor(8)
    vss = VoltageSupply(32)
    vextra = VoltageSupply(20)
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
    print(f'Number voltage positions = {nv}')
    print(r1)
    print(r1.component_type())
    print(r1.conductance())

if __name__ == '__main__':
    main()
