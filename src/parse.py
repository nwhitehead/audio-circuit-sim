
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
    n_pos: str
    n_neg: str
    value: float
    def component_type(self):
        return ComponentType.RESISTOR

@dataclass
class VoltageSource:
    n_pos: str
    n_neg: str
    value: float
    def component_type(self):
        return ComponentType.VOLTAGE_SOURCE

@dataclass
class CurrentSource:
    n_pos: str
    n_neg: str
    value: float
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

def parse_value(value):
    suffixes = {
        't': 1e12,
        'g': 1e9,
        'x': 1e6,
        'k': 1e3,
        'm': 1e-3,
        'u': 1e-6,
        'n': 1e-9,
        'p': 1e-12,
        'f': 1e-15,
    }
    for suffix in suffixes:
        if value.endswith(suffix) or value.endswith(suffix.upper()):
            left = value[:-1]
            return float(left) * suffixes[suffix]
    return float(value)

def test_parse_value():
    assert parse_value('12') == 12
    assert parse_value('1.2') == 1.2
    assert parse_value('1.2K') == 1200
    assert parse_value('1.2F') == 1.2e-15

def parse_type(part):
    if part.startswith('R'):
        return ComponentType.RESISTOR
    if part.startswith('V'):
        return ComponentType.VOLTAGE_SOURCE
    if part.startswith('I'):
        return ComponentType.CURRENT_SOURCE
    raise Exception('Unknown component type')

def parse_component(t, parts):
    if t == ComponentType.RESISTOR:
        return Resistor(n_pos=parts[0], n_neg=parts[1], value=parse_value(parts[2]))


def parse_top(txt, metadata):
    data = parse_lines(txt, metadata)
    nodes = []
    components = []
    for parts in data:
        t = parse_type(parts[0])
        print(t)
        component = parse_component(t, parts[1:])
    return data

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
