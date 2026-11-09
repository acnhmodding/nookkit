#!/usr/bin/env python3
"""Checks or applies the blank lines CODESTYLE asks for.

Inside code blocks (fn bodies, if/else, loops, unsafe, closures, match-arm blocks):
- a blank line before and after every multiline statement, `let ... else` included
- a blank line before a fn's final return expression when other statements precede it

Only ever inserts empty lines. Struct bodies and literals, match arm lists and items are left alone.

usage: check_spacing.py [--check] <file.rs>...
  --check  write nothing, list the missing blank lines and exit 1 if there are any
"""

import re
import sys

FN = re.compile(r'^(pub(\([^)]*\))? )?((const|async|unsafe|extern "C"|extern) )*fn\b|^fn\(')
RAW_STRING = re.compile(r'b?r(#*)"')
CHAR = re.compile(r"(\\(x[0-9a-fA-F]{2}|u\{[0-9a-fA-F]+\}|.)|[^\\'])'")


def lex(lines):
    """Per line: the brackets outside strings and comments as (column, char), and the code with strings blanked."""
    out = []
    block_comments = 0
    string = None

    for line in lines:
        events = []
        code = []
        i = 0

        while i < len(line):
            c = line[i]

            if block_comments:
                if line.startswith('*/', i):
                    block_comments -= 1
                    i += 2
                elif line.startswith('/*', i):
                    block_comments += 1
                    i += 2
                else:
                    i += 1

                continue

            if string == '"':
                if c == '\\':
                    i += 2
                elif c == '"':
                    string = None
                    i += 1
                else:
                    i += 1

                continue

            if string is not None:
                hashes = string
                if c == '"' and line[i + 1:i + 1 + hashes] == '#' * hashes:
                    string = None
                    i += 1 + hashes
                else:
                    i += 1

                continue

            if line.startswith('//', i):
                break

            if line.startswith('/*', i):
                block_comments += 1
                i += 2
                continue

            identifier_before = i > 0 and (line[i - 1].isalnum() or line[i - 1] == '_')

            raw = RAW_STRING.match(line, i)
            if raw and not identifier_before:
                string = len(raw.group(1))
                code.append('""')
                i = raw.end()
                continue

            if c == '"' or (line.startswith('b"', i) and not identifier_before):
                string = '"'
                code.append('""')
                i += 1 if c == '"' else 2
                continue

            if c == "'" or (line.startswith("b'", i) and not identifier_before):
                start = i + (1 if c == "'" else 2)
                char = CHAR.match(line, start)
                if char:
                    code.append("' '")
                    i = char.end()
                    continue

            if c in '{}()[]':
                events.append((i, c))

            code.append(c)
            i += 1

        out.append((events, ''.join(code).rstrip()))

    return out


def blocks(lexed):
    """(open, close) lines of every brace block opened at a line's end and closed at a line's start."""
    stack = []
    found = []

    for line, (events, code) in enumerate(lexed):
        for column, char in events:
            if char == '{':
                stack.append((line, column))
                continue

            if char != '}' or not stack:
                continue

            open_line, open_column = stack.pop()
            opener = lexed[open_line][1]
            if (
                open_line != line
                and open_column == len(opener) - 1
                and lexed[line][1].lstrip().startswith('}')
            ):
                found.append((open_line, line))

    return found


def kind(opener):
    """'fn' or 'code' for blocks holding statements, None for anything else"""
    head = opener.strip()[:-1].rstrip()
    stripped = re.sub(r'^\}\s*', '', head)

    if FN.search(stripped):
        return 'fn'

    if re.search(r'(\bunsafe|\belse|\bloop|\basync|\bmove|=>|\|)$', head):
        return 'code'

    if re.search(r'\bmatch\b', stripped):
        return None

    if re.match(r'^(else\s+)?(if|while|for)\b', stripped):
        return 'code'

    if re.search(r'(=|\(|,|^return)\s*(if|while|for)\s', stripped):
        return 'code'

    return None


def depth_delta(events):
    return sum(1 if char in '{([' else -1 for _, char in events)


def statements(lines, lexed, start, end):
    """The statements of lines[start:end] as (first line incl. comments, first code line, last line)"""
    out = []
    depth = 0
    first = None
    code_first = None

    for i in range(start, end):
        text = lines[i].strip()
        code = lexed[i][1].strip()

        if first is None:
            if text == '':
                continue

            first = i
            code_first = None

        depth += depth_delta(lexed[i][0])

        if code_first is None:
            if depth == 0 and text.startswith(('//', '#[', '#![')):
                continue

            code_first = i

        if depth != 0 or not code.endswith((';', '}')):
            continue

        follow = lines[i + 1].strip() if i + 1 < end else ''
        if follow.startswith(('.', '?', 'else')):
            continue

        out.append((first, code_first, i))
        first = None

    if first is not None:
        out.append((first, code_first if code_first is not None else first, end - 1))

    return out


def missing(text):
    """Line indices that need a blank line inserted before them"""
    lines = text.split('\n')
    lexed = lex(lines)
    found = set()

    for open_line, close_line in blocks(lexed):
        block = kind(lexed[open_line][1])
        if block is None:
            continue

        stmts = statements(lines, lexed, open_line + 1, close_line)

        for a, b in zip(stmts, stmts[1:]):
            adjacent = b[0] == a[2] + 1
            multiline = a[2] > a[1] or b[2] > b[1]
            if adjacent and multiline:
                found.add(b[0])

        if block == 'fn' and len(stmts) > 1:
            tail = stmts[-1]
            is_expression = not lexed[tail[2]][1].strip().endswith(';')
            if is_expression and lines[tail[0] - 1].strip() != '':
                found.add(tail[0])

    return sorted(found)


def main(args):
    check = '--check' in args
    paths = [arg for arg in args if arg != '--check']
    total = 0

    for path in paths:
        with open(path) as f:
            text = f.read()

        lines = missing(text)
        if not lines:
            continue

        total += len(lines)

        if check:
            for line in lines:
                print(f'{path}:{line + 1}: missing blank line before this line')

            continue

        out = text.split('\n')
        for line in reversed(lines):
            out.insert(line, '')

        with open(path, 'w') as f:
            f.write('\n'.join(out))

        print(f'{path}: {len(lines)} blank lines added')

    if check and total:
        print(f'{total} missing blank lines, run tools/check_spacing.py on the files to add them')
        return 1

    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
