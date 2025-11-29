"""

Command line utility for parsing KiCAD lib files and outputting JSON format.

Example usage:

    uv run script/lib2json.py ../kicad-symbols/Transistor_BJT.lib > BJT.json

"""

import argparse
import json

PAIRS = {
    'DEF': 'ENDDEF', 
    'DRAW': 'ENDDRAW', 
    '$FPLIST': '$ENDFPLIST'
}

def is_comment(line):
    return line.lstrip().startswith('#')

def parse_line(line):
    line = line.lstrip()
    res = []
    for part in line.split():
        try:
            v = float(part)
        except ValueError:
            if part.startswith('"') and part.endswith('"'):
                v = part.encode().decode('unicode-escape').lstrip('"').rstrip('"')
            else:
                v = part
        res.append(v)
    return res

def parse_lines(lines):
    res = [[]]
    envs = []
    for line in lines:
        if not is_comment(line):
            v = parse_line(line)
            if len(v) > 0 and v[0] in PAIRS:
                name = v[0]
                res.append([])
                envs.append(name)
            elif len(v) > 0 and len(envs) > 0 and v[0] == PAIRS[envs[-1]]:
                name = envs.pop()
                contents = res.pop()
                res[-1].append([name, contents])
            elif len(v) > 0 and v[0] == 'EESchema-LIBRARY':
                # Just ignore schema version for now
                pass
            else:
                res[-1].append(v)
    return res[0]

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('input')
    args = parser.parse_args()
    with open(args.input, 'rt') as fin:
        lines = [ line.rstrip() for line in fin]
        data = parse_lines(lines)
        print(json.dumps(data))
