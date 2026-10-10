#!/usr/bin/env python3
"""Worldwright alpha: exercise the actual compiled CLI and desktop binary.

Runs on a build runner, without network access or a window system. This is
a minimum headless/native-binary gate, NOT a substitute for a human viewport
acceptance session. The egui workspace tests cover UI state transitions.
"""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REQUIRED_COMMANDS = (
    "new", "open", "qsave", "saveas", "undo", "redo",
    "line", "circle", "layer.new", "layer.current",
    "nurbs.curve3d", "geometry3d.list", "geometry3d.preview",
    "geometry3d.transform", "geometry3d.pick", "geometry3d.select",
    "geometry3d.snap", "mesh3d.create", "mesh3d.list",
    "mesh3d.edit", "model.create", "model.list",
    "worldwright.tool.list", "worldwright.tool.run",
    "worldwright.history.create", "worldwright.history.list",
    "buildercraft.capabilities",
)

def run(binary: Path, *args: str, success: bool = True) -> str:
    proc = subprocess.run(
        [str(binary), *args], text=True, capture_output=True,
        timeout=45, check=False,
    )
    if success and proc.returncode:
        raise AssertionError(
            f"{binary.name} {args!r} exited {proc.returncode}\n"
            f"stdout:\n{proc.stdout[-3000:]}\nstderr:\n{proc.stderr[-3000:]}"
        )
    if not success and proc.returncode == 0:
        raise AssertionError(f"Invalid command unexpectedly succeeded: {args!r}")
    return proc.stdout

def command(binary: Path, path: Path, command_id: str) -> dict:
    raw = run(binary, "run", str(path), "--cmd", f"{command_id} {{}}")
    try:
        value = json.loads(raw.strip().splitlines()[-1])
    except (ValueError, IndexError) as exc:
        raise AssertionError(f"Bad {command_id} output: {raw[-500:]}") from exc
    if not isinstance(value, dict):
        raise AssertionError(f"{command_id} result must be an object")
    return value

def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: alpha_compiled_smoke.py /path/to/cadcraft-cli /path/to/cadcraft")
    cli, desktop = (Path(arg).resolve() for arg in sys.argv[1:])
    for executable in (cli, desktop):
        if not executable.is_file():
            raise AssertionError(f"Native executable does not exist: {executable}")
        version = run(executable, "--version")
        if not version.strip():
            raise AssertionError(f"Missing compiled version output: {executable}")

    catalog = json.loads(run(cli, "commands"))
    available = {item["id"] for item in catalog}
    missing = sorted(set(REQUIRED_COMMANDS) - available)
    if missing:
        raise AssertionError(f"Alpha command registry missing: {missing}")

    with tempfile.TemporaryDirectory(prefix="worldwright-alpha-") as tmp:
        native = Path(tmp) / "AlphaRoundtrip.dftba"
        legacy = Path(tmp) / "AlphaLegacy.bcraft"
        nurbs = {
            "name": "Alpha curve",
            "curve": {
                "degree": 1,
                "control": [
                    {"x": 0.0, "y": 0.0, "z": 0.0},
                    {"x": 8.0, "y": 0.0, "z": 4.0},
                ],
                "weights": [1.0, 1.0],
                "knots": [0.0, 0.0, 1.0, 1.0],
            },
        }
        triangle = {
            "name": "Alpha triangle",
            "mesh": {
                "vertices": [
                    {"x": 0.0, "y": 0.0, "z": 0.0},
                    {"x": 5.0, "y": 0.0, "z": 0.0},
                    {"x": 0.0, "y": 5.0, "z": 0.0},
                ],
                "faces": [{"triangle": [0, 1, 2]}],
            },
        }
        def cmd(name: str, params: dict) -> tuple[str, str]:
            return "--cmd", f"{name} {json.dumps(params, separators=(',', ':'))}"
        run(
            cli, "run", "--metric",
            *cmd("layer.new", {"name": "Alpha Layer", "current": True}),
            *cmd("line", {"points": [[0, 0], [8, 5]]}),
            *cmd("circle", {"center": [3, 3], "radius": 2}),
            *cmd("nurbs.curve3d", nurbs),
            *cmd("mesh3d.create", triangle),
            "--save", str(native),
        )
        if not native.is_file() or native.stat().st_size < 64:
            raise AssertionError("Native .dftba project was not written")

        mesh_list = command(cli, native, "mesh3d.list")
        geom_list = command(cli, native, "geometry3d.list")
        if len(mesh_list.get("objects", [])) != 1:
            raise AssertionError("Native project lost its polygon mesh on reopen")
        if len(geom_list.get("objects", [])) != 1:
            raise AssertionError("Native project lost its rational curve on reopen")
        if mesh_list["objects"][0]["name"] != "Alpha triangle":
            raise AssertionError("Native mesh name changed during roundtrip")

        # Saving the reopened source in the legacy extension must not
        # irreversibly strip the exact curve or polygon data.
        run(cli, "run", str(native), "--save", str(legacy))
        if not legacy.is_file():
            raise AssertionError(".bcraft legacy project save failed")
        if len(command(cli, legacy, "mesh3d.list").get("objects", [])) != 1:
            raise AssertionError("Legacy .bcraft compatibility lost mesh content")
        if len(command(cli, legacy, "geometry3d.list").get("objects", [])) != 1:
            raise AssertionError("Legacy .bcraft compatibility lost NURBS content")

        run(
            cli, "run", "--metric",
            *cmd("mesh3d.create", {
                "name": "Invalid",
                "mesh": {"vertices": triangle["mesh"]["vertices"], "faces": [{"triangle": [0, 0, 0]}]},
            }),
            success=False,
        )
        paired = command(cli, native, "worldwright.tool.list")
        if not paired.get("paired_tools"):
            raise AssertionError("CAD and OrbWeaver shared tool catalog is empty")

    print(f"PASS: compiled CLI and GUI versions; {len(REQUIRED_COMMANDS)} command IDs; "
          "2D + exact NURBS + polygon scene; .dftba/.bcraft roundtrip; "
          "hostile mesh rejection; shared OrbWeaver tool registry")

if __name__ == "__main__":
    main()
