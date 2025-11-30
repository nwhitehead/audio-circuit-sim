
import argparse
from dataclasses import dataclass
from enum import Enum

class ComponentType(Enum):
    RESISTOR = 1
    VOLTAGE_SOURCE = 2
    CURRENT_SOURCE = 3
    NONLINEAR = 4

@dataclass
class Resistor:
    r: float
    def component_type(self):
        return ComponentType.RESISTOR
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

def is_comment(line):
    return line == '' or line.startswith('#')

def is_end(line):
    return line == '.end'

def parse_lines(txt, metadata):
    result = []
    for line in txt.splitlines():
        s_line = line.lstrip().rstrip()
        if is_end(line):
            break
        if not is_comment(s_line):
            result.append(s_line.split(' '))
    return result

def parse_resistor(parts):
    pass

def parse_type(line):
    if line.startswith('R'):
        return ComponentType.RESISTOR
    if line.startswith('V'):
        return ComponentType.VOLTAGE_SOURCE
    if line.startswith('I'):
        return ComponentType.CURRENT_SOURCE
    raise Exception('Unknown component type')

def parse_top(txt, metadata):
    data = parse_lines(txt, metadata)
    nodes = []
    components = []
    for c in data:
        pass

def main():
    parser = argparse.ArgumentParser(description='This utility is an RCR file parser')
    parser.add_argument('filename', nargs='+')
    args = parser.parse_args()
    for filename in args.filename:
        with open(filename, 'r') as f:
            txt = f.read()
            data = parse_top(txt, { filename })
            print(data)

if __name__ == '__main__':
    main()
