#!/usr/bin/env python3
#
# check_boot_contract.py - verify the BOOT layout constants in the Rust board module
#
# Author:  Sandesh Ghimire <sandesh@soccentric.com>
# Usage:   scripts/check_boot_contract.py [--source PATH]
#
# Fails when src/board/mod.rs and the BOOT layout have drifted.  The values
# are the binary contract shared with the bootloader (LAY-001..LAY-005,
# HND-003, Appendix B); change both sides together.
#

import argparse
import pathlib
import re
import sys

EXPECTED = {
    "SLOT_BASE": 0x0800C000,
    "SLOT_SIZE": 0x000F4000,
    "FLASH_BASE": 0x08000000,
    "META_BASE": 0x08008000,
    "CONFIRMED_RECORD_OFFSET": 0x140,
    "CONFIRMED_MAGIC": 0x434F4E46,
}

PATTERN = re.compile(
    r"^\s*pub\s+const\s+([A-Z0-9_]+)\s*:\s*[a-z0-9]+\s*=\s*(0x[0-9A-Fa-f_]+|\d+)\s*;"
)


def read_constants(path: pathlib.Path) -> dict[str, int]:
    constants: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = PATTERN.match(line)
        if match:
            constants[match.group(1)] = int(match.group(2).replace("_", ""), 0)
    return constants


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--source",
        default="src/board/mod.rs",
        help="board contract source (default: %(default)s)",
    )
    args = parser.parse_args()

    path = pathlib.Path(args.source)
    if not path.is_file():
        print(f"boot contract: {path} not found", file=sys.stderr)
        return 1

    constants = read_constants(path)
    failed = False
    for name, want in EXPECTED.items():
        if name not in constants:
            print(f"boot contract: {name} is missing from {path}", file=sys.stderr)
            failed = True
        elif constants[name] != want:
            print(
                f"boot contract: {name} is 0x{constants[name]:08X}, "
                f"expected 0x{want:08X}",
                file=sys.stderr,
            )
            failed = True

    if failed:
        return 1
    print("boot contract: slot, metadata and record offsets match the BOOT layout")
    return 0


if __name__ == "__main__":
    sys.exit(main())
