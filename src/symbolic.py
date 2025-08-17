"""
Symbolic equation solving

"""

from dataclasses import dataclass
from enum import Enum
import numpy as np

existing_variables = set()
fresh_id = 0

@dataclass
class Variable:
    name: str
    def __init__(self, name_request=None):
        global fresh_id
        global existing_variables
        if name_request is None:
            name_request = 'v'
        name = name_request
        while name in existing_variables:
            name = f'{name_request}{fresh_id}'
            fresh_id += 1
        self.name = name
        existing_variables.add(name)

def main():
    x = Variable()
    y = Variable()
    z = Variable()
    print(x, y, z)

if __name__ == '__main__':
    main()
