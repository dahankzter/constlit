#!/usr/bin/env python3
"""Generates constlit's Linux errno tables from the libc crate's source.

Usage: errno_tables.py <path to libc-0.2.191/src>

Prints two Rust fragments: the runtime tables (constlit/src/linux/errno.rs) and
the macro crate's name list and presence (constlit-macros/src/errno.rs). Each
family's table is linux_l4re_shared.rs plus the family's architecture file,
keeping `pub const E…: c_int = N;` with 1 <= N <= 1200 (mips' EDQUOT is 1133) and
dropping EPOLL*, EFD_*, EXT*, EOF and the x86 register indices EFLAGS and ES.
Every number must belong to one name, apart from the EDEADLOCK alias. The static asserts against libc in
constlit/tests/errno_libc.rs are what verify the result.
"""
import re
import sys

FAMILIES = [
    ("GENERIC", "linux/gnu/b64/x86_64/mod.rs"),
    ("POWERPC", "linux/gnu/b64/powerpc64/mod.rs"),
    # mips64's file holds every mips errno at top level; the 32-bit mips file keeps
    # some inside a cfg_if block. The numbers are the same kernel ABI.
    ("MIPS", "linux/gnu/b64/mips64/mod.rs"),
    ("SPARC", "linux/gnu/b64/sparc64/mod.rs"),
]
ALIASES = {"EWOULDBLOCK", "ENOTSUP", "EDEADLOCK"}
NOT_ERRNO = {"EOF", "EFLAGS", "ES"}
PATTERN = re.compile(r"^pub const (E[A-Z0-9]+): c_int = (0x[0-9a-fA-F]+|\d+);", re.M)


def read(root, path):
    found = {}
    for name, value in PATTERN.findall(open(f"{root}/unix/linux_like/{path}").read()):
        if name.startswith(("EPOLL", "EFD_", "EXT")) or name in NOT_ERRNO:
            continue
        number = int(value, 0)
        if 1 <= number <= 1200:
            found.setdefault(name, number)
    return found


def main():
    root = sys.argv[1]
    shared = read(root, "linux_l4re_shared.rs")
    tables = {}
    for family, path in FAMILIES:
        table = dict(shared)
        table.update(read(root, path))
        numbers = [n for name, n in table.items() if name not in ALIASES]
        assert len(numbers) == len(set(numbers)), (family, sorted(numbers))
        tables[family] = table
    names = []
    for family, _ in FAMILIES:
        for name in sorted(tables[family], key=lambda n: (tables[family][n], n in ALIASES, n)):
            if name not in names:
                names.append(name)
    print("// ---- runtime tables ----")
    for family, _ in FAMILIES:
        table = tables[family]
        # Canonical names first for each number, then aliases.
        rows = sorted(table.items(), key=lambda kv: (kv[1], kv[0] in ALIASES, kv[0]))
        print(f"const {family}: [(&str, i32); {len(rows)}] = [")
        for name, number in rows:
            print(f'    ("{name}", {number}),')
        print("];")
    print("// ---- macro names ----")
    print(f"pub const NAMES: [&str; {len(names)}] = [")
    for name in names:
        print(f'    "{name}",')
    print("];")
    # Every family has every name, so errno! needs no per-family presence; if a
    # future libc breaks this, regenerating fails here rather than mis-numbering.
    for family, _ in FAMILIES:
        assert set(tables[family]) == set(names), (family, set(names) - set(tables[family]))
    # The generic number of each name, for "2 on most Linux targets" hints.
    print(f"const GENERIC_NUMBERS: [i32; {len(names)}] = [")
    for name in names:
        print(f"    {tables['GENERIC'][name]},")
    print("];")


main()
