#!/usr/bin/env python3
"""Fail closed on undocumented third-party UI source reuse.

Only records *actual copied code* after a precise source revision and license
notice are recorded. Behavioral inspiration may be used independently without
claiming the upstream source was copied or licensed for redistribution.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

MANIFEST = Path(__file__).resolve().parents[1] / "docs/dependencies/UI_CODE_REUSE.json"
MODES = {"source_copy", "dependency", "reference_only", "existing_foundation"}
ACCESS = {"source_available", "unverified"}


def validate_manifest(doc: object) -> list[str]:
    errors: list[str] = []
    if not isinstance(doc, dict) or doc.get("schema_version") != 1:
        return ["expected schema_version 1 JSON object"]
    entries = doc.get("source_entries")
    if not isinstance(entries, list) or not entries:
        return ["source_entries must be a non-empty array"]
    seen: set[str] = set()
    for index, entry in enumerate(entries):
        where = f"source_entries[{index}]"
        if not isinstance(entry, dict):
            errors.append(f"{where}: expected object")
            continue
        identifier = entry.get("id")
        if not isinstance(identifier, str) or not identifier.strip():
            errors.append(f"{where}: missing id")
        elif identifier in seen:
            errors.append(f"{where}: duplicate id {identifier}")
        else:
            seen.add(identifier)
        mode = entry.get("reuse_mode")
        if mode not in MODES:
            errors.append(f"{where}: unknown reuse_mode")
            continue
        access = entry.get("source_access")
        if access not in ACCESS:
            errors.append(f"{where}: invalid source_access")
        url = entry.get("url")
        if not isinstance(url, str) or not url.startswith("https://"):
            errors.append(f"{where}: source URL must use HTTPS")
        evidence = entry.get("license_evidence")
        if not isinstance(evidence, str) or not evidence.startswith("https://"):
            errors.append(f"{where}: license evidence URL required")
        license_id = entry.get("license")
        if not isinstance(license_id, str) or not license_id.strip():
            errors.append(f"{where}: declared license required")
        destinations = entry.get("destinations")
        if not isinstance(destinations, list) or any(not isinstance(p, str) or not p.strip() for p in destinations):
            errors.append(f"{where}: destinations must be a list of non-empty paths")
            continue
        if mode == "reference_only":
            if destinations:
                errors.append(f"{where}: reference-only entry must not claim copied destinations")
        elif mode in {"source_copy", "dependency"}:
            if access != "source_available" or str(license_id).startswith("UNVERIFIED"):
                errors.append(f"{where}: unverified/proprietary source cannot be incorporated")
            if not isinstance(entry.get("source_revision"), str) or len(entry["source_revision"]) < 7:
                errors.append(f"{where}: copied/dependency source requires pinned revision")
            if mode == "source_copy" and not destinations:
                errors.append(f"{where}: copied source must list destination paths")
            if mode == "source_copy" and not entry.get("notices"):
                errors.append(f"{where}: copied source must list license notice paths")
        elif mode == "existing_foundation":
            if not destinations:
                errors.append(f"{where}: existing foundation should identify local directories")
        if not isinstance(entry.get("decision"), str) or not entry["decision"].strip():
            errors.append(f"{where}: reuse rationale required")
    return errors


def main() -> int:
    try:
        with MANIFEST.open(encoding="utf-8") as handle:
            doc = json.load(handle)
    except (OSError, ValueError) as exc:
        print(f"UI source reuse manifest unavailable or invalid: {exc}", file=sys.stderr)
        return 1
    errors = validate_manifest(doc)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"UI source-reuse manifest valid: {len(doc['source_entries'])} source records")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
