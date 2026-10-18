#!/usr/bin/env python3
"""One-off, deterministic reconciliation of archived #67–#73 onto current main.

Run only after merging pinned a399caa... with --no-commit. Preserves the
newer mainline exact-solid transform, Haiku resource policy, and OrbWeaver work.
Never use this as a general conflict resolver.
"""
from pathlib import Path
import json
import re
import subprocess

def resolve(name, policy):
    path = Path(name)
    text = path.read_text()
    conflicts = 0
    while "<<<<<<< HEAD" in text:
        start = text.index("<<<<<<< HEAD")
        middle = text.index("=======", start)
        end = text.index(">>>>>>> stack-old", middle)
        ours = text[start + len("<<<<<<< HEAD"):middle].lstrip("\n")
        theirs = text[middle + len("======="):end].lstrip("\n")
        replacement = policy(ours, theirs)
        text = text[:start] + replacement + text[end + len(">>>>>>> stack-old"):]
        conflicts += 1
    if not conflicts:
        raise RuntimeError(f"Expected an audited conflict: {name}")
    path.write_text(text)
    print(f"Resolved {conflicts} conflicts: {name}")

resolve(".github/workflows/brep-document-integration.yml", lambda ours, theirs: ours + theirs)
resolve(".github/workflows/google-drive-source-backup.yml", lambda ours, theirs: ours)
resolve("docs/dependencies/kernel-operation-deps.json", lambda ours, theirs: theirs)
resolve("crates/engine/src/cmd/worldwright_tools.rs", lambda ours, theirs: theirs)
resolve("crates/kernel/src/shared_tools.rs", lambda ours, theirs: theirs)
resolve("crates/kernel/src/lib.rs", lambda ours, theirs: ours + theirs)
resolve("evaluations/cadrum/src/adapter.rs", lambda ours, theirs: ours + "    }\n\n" + theirs)
resolve("evaluations/cadrum/src/worker_protocol.rs", lambda ours, theirs: ours + "        }\n    }\n    #[test]\n" + theirs)

# Retain BOTH the newer transform regression tests and archived swept-solid
# tests. Never accept one side's test module wholesale.
path = Path("crates/engine/src/cmd/brep3d.rs")
mixed = path.read_text()
marker = "#[cfg(test)]\nmod tests {"
if marker not in mixed:
    raise RuntimeError("Missing exact BRep test module")
prefix = mixed[:mixed.index(marker)]
read = lambda ref: subprocess.check_output(
    ["git", "show", f"{ref}:crates/engine/src/cmd/brep3d.rs"], text=True
)
base = read("HEAD")
archived = read("stack-old")
original_tests = base[base.index(marker):]
old_tests = archived[archived.index(marker) + len(marker):]
functions = re.compile(r"(?m)^    fn (\w+)\(")
already = set(functions.findall(original_tests))
new_cases = []
for match in functions.finditer(old_tests):
    if match.group(1) in already:
        continue
    start = match.start()
    while start > 0:
        prev_end = start - 1
        prev_start = old_tests.rfind("\n", 0, prev_end) + 1
        line = old_tests[prev_start:prev_end + 1]
        if re.match(r"^    #\[.*\]$", line.rstrip("\n")):
            start = prev_start
        else:
            break
    following = functions.search(old_tests, match.end())
    end = following.start() if following else len(old_tests)
    if following:
        while end > 0:
            prev_start = old_tests.rfind("\n", 0, end - 1) + 1
            line = old_tests[prev_start:end].strip()
            if line.startswith("#["):
                end = prev_start
            else:
                break
    case = old_tests[start:end]
    if following is None:
        close = case.rfind("\n}")
        if close >= 0:
            case = case[:close]
    new_cases.append((match.group(1), case))
last = original_tests.rfind("\n}")
if last <= 0:
    raise RuntimeError("Malformed BRep tests module")
combined = prefix + original_tests[:last] + "\n"
for _, case in new_cases:
    combined += case if case.endswith("\n") else case + "\n"
combined += original_tests[last:]
path.write_text(combined)
print("Kept archived BRep acceptance tests:", [name for name, _ in new_cases])

# Merge per-operation declarations, not outdated summary constants. The tool
# catalog is the authoritative static paired contract count.
plan = json.loads(Path("docs/dependencies/tool-groups.json").read_text())
deps_path = Path("docs/dependencies/kernel-operation-deps.json")
deps = json.loads(deps_path.read_text())
paired = len(plan["paired_operations"])
deps["counts"]["total"] = len(deps["entries"])
deps["counts"]["paired"] = paired
deps["counts"]["pending_source_review"] = len(deps["entries"]) - deps["counts"]["explicit_dependencies_mapped"]
deps_path.write_text(json.dumps(deps, indent=2) + "\n")
for name in ["crates/kernel/src/shared_tools.rs", "crates/engine/src/cmd/worldwright_tools.rs"]:
    path = Path(name)
    body = path.read_text().replace("SHARED_TOOLS.len(), 41", f"SHARED_TOOLS.len(), {paired}")
    body = body.replace("Some(41)", f"Some({paired})")
    path.write_text(body)
if paired != 42 or len(deps["entries"]) != 74:
    raise RuntimeError(f"Unexpected operation inventory: {paired} paired / {len(deps['entries'])} total")

workflow = Path(".github/workflows/google-drive-source-backup.yml")
workflow.write_text(workflow.read_text().rstrip() + "\n")

conflicted = subprocess.run(["git", "diff", "--name-only", "--diff-filter=U"], capture_output=True, text=True, check=True)
for name in conflicted.stdout.splitlines():
    if "<<<<<<<" in Path(name).read_text():
        raise RuntimeError(f"Unresolved textual conflict: {name}")
print("Merge reconciliation complete; validate with static gate and Rust CI.")
