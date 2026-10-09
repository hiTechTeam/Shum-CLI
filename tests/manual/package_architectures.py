#!/usr/bin/env python3
"""Regression checks for fat slices, deployment targets and embedded BLE architecture."""
from pathlib import Path
import struct
import sys
import tempfile
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from package_macos import macho_requirements

def image(cpu, minimum=15):
    load = struct.pack("<6I", 0x32, 24, 1, minimum << 16, 15 << 16, 0)
    return struct.pack("<8I", 0xfeedfacf, cpu, 0, 2, 1, len(load), 0, 0) + load

def universal(arm, intel):
    header = b"\xca\xfe\xba\xbe" + struct.pack(">I", 2)
    first = 48
    second = first + len(arm)
    header += struct.pack(">5I", 0x100000C, 0, first, len(arm), 0)
    header += struct.pack(">5I", 0x1000007, 0, second, len(intel), 0)
    return header + arm + intel

with tempfile.TemporaryDirectory(prefix="shum-macho-test-") as temporary:
    path = Path(temporary) / "shum"
    arm = image(0x100000C)
    intel = image(0x1000007)
    path.write_bytes(universal(arm + arm, intel + intel))
    minima, images = macho_requirements(path)
    assert minima == {"arm64": "15.0.0", "x86_64": "15.0.0"}
    assert len(images) == 4
    assert images[0]["offset"] == 48
    for invalid in [arm + arm, universal(arm + arm, intel + arm),
                    universal(arm + arm, image(0x1000007, 16) + intel),
                    universal(arm + arm, intel)]:
        path.write_bytes(invalid)
        try:
            macho_requirements(path)
        except ValueError:
            pass
        else:
            raise AssertionError("invalid architecture/deployment target accepted")
print("PASS: universal slices, per-architecture macOS minima, matching embedded BLE")
print("PASS: thin input, wrong helper architecture, newer minimum OS and missing helper rejected")
