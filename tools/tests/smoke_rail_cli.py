#!/usr/bin/env python3
"""Smoke three real Worldwright native CLI command paths with typed JSON.
Run after: cargo build --locked -p cadcraft-cli. No OpenSCAD/GPU/network needed.
"""
import json
import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
EXE = ROOT / "target" / "debug" / ("cadcraft-cli.exe" if os.name == "nt" else "cadcraft-cli")

def tagged(kind, value):
    return {"kind": kind, "value": value}

def p(x, y, z):
    return {"x": x, "y": y, "z": z}

def call(id, inputs):
    argument = id + " " + json.dumps({"inputs": inputs}, separators=(",", ":"))
    run = subprocess.run(
        [str(EXE), "run", "--metric", "--cmd", argument],
        cwd=ROOT, capture_output=True, text=True, timeout=30, check=True,
    )
    records = [json.loads(line) for line in run.stdout.splitlines() if line.strip().startswith("{")]
    assert len(records) == 1, (id, run.stdout, run.stderr)
    output = records[0]
    assert output["operation"].startswith("kernel."), output
    value = output["output"]
    assert value["kind"] == "mesh", (id, value)
    assert len(value["value"]["vertices"]) >= 6
    assert len(value["value"]["faces"]) >= 3
    return value["value"]

RAIL = tagged("polyline", [p(0, 0, 0), p(6, 0, 0)])
UP = tagged("vector", p(0, 0, 1))
pipe = call("worldwright.pipe", {
    "rail": RAIL, "up": UP, "start_radius": tagged("number", 1.0),
    "end_radius": tagged("number", 1.0), "wall": tagged("number", 0.0),
    "stations": tagged("count", 4), "sides": tagged("count", 12),
    "flat_caps": tagged("count", 1),
})
assert len(pipe["vertices"]) == 50
sweep1 = call("worldwright.sweep1", {
    "rail": RAIL, "up": UP,
    "profile": tagged("polyline", [p(0, 1, 0), p(0, 0, 1), p(0, -1, 0), p(0, 0, -1)]),
    "stations": tagged("count", 4), "closed_profile": tagged("count", 1),
})
assert len(sweep1["faces"]) == 12
sweep2 = call("worldwright.sweep2", {
    "rail_a": RAIL,
    "rail_b": tagged("polyline", [p(0, 2, 0), p(6, 2, 0)]),
    "section": tagged("polyline", [p(0, 0, 0), p(0.5, 1, 0), p(1, 0, 0)]),
    "stations": tagged("count", 4),
})
assert len(sweep2["faces"]) == 6
print("Worldwright native CLI: Pipe, Sweep1 and Sweep2 executed and returned mesh topology.")
