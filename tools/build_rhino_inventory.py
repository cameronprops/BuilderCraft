#!/usr/bin/env python3
"""Build a factual command-name inventory from locally downloaded official indexes.

No help text, icons or proprietary implementation is copied. This inventories
documented Rhino 8 commands, not installed plug-in or undocumented commands.
Run with --index windows=PATH --index mac=PATH --output docs/commands/rhino8.json.
Existing records retain reviewed status/options/evidence on refresh.
"""
import argparse
import hashlib
import json
import re
from datetime import date
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urljoin

SOURCES = {
    "windows": "https://docs.mcneel.com/rhino/8/help/en-us/commandlist/command_list.htm",
    "mac": "https://docs.mcneel.com/rhino/8mac/help/en-us/commandlist/command_list.htm",
}
STATUSES = {"working", "partial", "unvalidated", "not_implemented", "not_applicable"}


class CommandIndex(HTMLParser):
    def __init__(self):
        super().__init__()
        self.in_heading = False
        self.link = None
        self.commands = []

    def handle_starttag(self, tag, attrs):
        if tag == "h5":
            self.in_heading = True
        if tag == "a" and self.in_heading:
            self.link = [dict(attrs).get("href", ""), ""]

    def handle_data(self, value):
        if self.link is not None:
            self.link[1] += value

    def handle_endtag(self, tag):
        if tag == "a" and self.link is not None:
            href, label = self.link
            name = label.strip()
            if "commands/" in href and re.fullmatch(r"[A-Za-z0-9_]+", name):
                self.commands.append((name, href))
            self.link = None
        if tag == "h5":
            self.in_heading = False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--index", action="append", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--candidate-root", type=Path)
    args = parser.parse_args()
    old = {}
    if args.output.exists():
        old = {r["name"]: r for r in json.loads(args.output.read_text())["commands"]}
    candidates = {}
    if args.candidate_root:
        for path in (args.candidate_root / "crates/engine/src/cmd").glob("*.rs"):
            for command_id, label in re.findall(r'CommandSpec::new\(\s*"([^"\n]+)"\s*,\s*"([^"\n]+)"', path.read_text()):
                candidates.setdefault(command_id.lower(), command_id)
                candidates.setdefault(label.lower(), command_id)
    records = {}
    snapshots = []
    for item in args.index:
        platform, filename = item.split("=", 1)
        if platform not in SOURCES:
            parser.error("platform must be windows or mac")
        raw = Path(filename).read_bytes()
        index = CommandIndex()
        index.feed(raw.decode("utf-8"))
        if len(index.commands) < 500:
            raise ValueError("Unexpected index structure; refuse incomplete inventory")
        snapshots.append({"platform": platform, "url": SOURCES[platform],
                          "sha256": hashlib.sha256(raw).hexdigest(),
                          "unique_commands": len({n for n, _ in index.commands})})
        for name, href in index.commands:
            candidate = candidates.get(name.lower())
            entry = records.setdefault(name, {
                "name": name, "platforms": [], "references": [],
                "status": "unvalidated" if candidate else "not_implemented",
                "candidate_command": candidate,
                "owner": "cad", "options": [], "evidence": [],
                "notes": "Name match only; Rhino behavior and options not verified." if candidate else "Native equivalent not mapped yet.",
            })
            if platform not in entry["platforms"]:
                entry["platforms"].append(platform)
            reference = urljoin(SOURCES[platform], href)
            if reference not in entry["references"]:
                entry["references"].append(reference)
    for name, entry in records.items():
        prior = old.get(name)
        if prior:
            for key in ("status", "candidate_command", "owner", "options", "evidence", "notes"):
                if key in prior:
                    entry[key] = prior[key]
        if entry["status"] not in STATUSES:
            raise ValueError("Invalid status for " + name)
        if entry["status"] in {"working", "partial"} and not entry["evidence"]:
            raise ValueError("Acceptance evidence required for " + name)
    removed = [old[n] for n in sorted(set(old) - set(records))]
    result = {"schema_version": 1, "reference_product": "Rhino 8",
              "retrieved_at": date.today().isoformat(),
              "scope": "Documented Windows/Mac command quick-reference headings; plug-in and undocumented commands require a separate runtime audit.",
              "source_snapshots": snapshots, "command_count": len(records),
              "commands": sorted(records.values(), key=lambda r: r["name"].lower()),
              "removed_from_reference": removed}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"command_count": len(records), "status_counts": {
        s: sum(r["status"] == s for r in records.values()) for s in sorted(STATUSES)}}))


if __name__ == "__main__":
    main()
