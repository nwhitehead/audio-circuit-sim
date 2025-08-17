"""
Symbolic equation solving

"""

from __future__ import annotations
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
    def __str__(self):
        return self.name

@dataclass
class VarExpr:
    v: Variable
    def __str__(self):
        return f'{self.v}'

class BinOp(Enum):
    PLUS = 1
    MINUS = 2
    TIMES = 3
    DIVIDE = 4
    def __str__(self):
        match self:
            case BinOp.PLUS:
                return '+'
            case BinOp.MINUS:
                return '-'
            case BinOp.TIMES:
                return '*'
            case BinOp.DIVIDE:
                return '/'
            case _:
                raise ValueError()

@dataclass
class BinExpr:
    op: str
    left: Expr
    right: Expr
    def __str__(self):
        return f'({self.left} {self.op} {self.right})'

@dataclass
class ZeroExpr:
    def __str__(self):
        return '0'

Expr = ZeroExpr | VarExpr | BinExpr

def main():
    x = Variable()
    y = Variable()
    z = Variable()
    assert(f'{x} {y} {z}' == 'v v0 v1')
    assert(x == x)
    assert(x != y)
    v = VarExpr(x)
    print(v)
    v0 = BinExpr(BinOp.PLUS, VarExpr(x), VarExpr(y))
    v1 = BinExpr(BinOp.DIVIDE, v0, VarExpr(x))
    print(v1)

if __name__ == '__main__':
    main()
