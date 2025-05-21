import argparse

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
                print(f'OPEN {name}')
                res.append([])
                envs.append(name)
            elif len(v) > 0 and len(envs) > 0 and v[0] == PAIRS[envs[-1]]:
                name = envs.pop()
                print(f'CLOSE {name}')
                contents = res.pop()
                print(f'Contents is {contents}')
                print(f'res = {res}')
                res[-1].append({
                    'name': name,
                    'content': contents,
                })
            else:
                res[-1].append(v)
        print(res)
    return res

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('input')
    args = parser.parse_args()
    print(args)
    with open(args.input, 'rt') as fin:
        lines = [ line.rstrip() for line in fin]
        print(parse_lines(lines))
                # if len(v) > 0 and v[0] in PAIRS:
                #     print(f'OPEN {v[0]}')
