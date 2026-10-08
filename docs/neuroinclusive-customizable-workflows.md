# BuilderCraft: Neuroinclusive and User-Customizable Workflows

Status: approved design requirements, not a claim of completed UI functionality.

## Principle

BuilderCraft must be approachable for diverse cognitive, sensory, motor and
attention needs. Users customize how work is presented, sequenced and controlled
without changing project geometry, data or team interoperability.
No diagnostic label or single preset should dictate how a person works.
Make every accessibility and cognitive-load feature available to any user.

## Adjustable interface, predictable behavior

- Adjustable density: Focus, Standard and Expert layouts; customizable panels,
  toolbar positions, text/icon sizes, spacing and visible metadata.
- Minimize motion and visual noise: optional reduced animation, no flashing UI,
  adjustable highlight strength, muted palettes, optional sound/haptic feedback,
  high-contrast and accessible color modes; never rely on color alone.
- Stable controls: consistent command naming, predictable focus/tab order,
  deterministic hotkeys, configurable input methods and a clear undo history.
- Modality flexibility: command line, buttons, keyboard shortcuts, direct
  manipulation, mouse/pen/touch, guided dialogs, scripted and graph actions.
- Progressive disclosure: simple tasks do not expose every parameter;
  advanced properties can always be opened without hidden destructive defaults.
- Spatial continuity: preserve camera, selection, tool settings and layout
  when switching discipline workspaces unless the user explicitly opts out.
- Avoid intrusive dialog chains, auto-opening panels, surprise focus steals,
  forced tutorials, and destructive context-dependent shortcuts.

## Task flow, memory and interruption recovery

- Optional task-oriented workflows: step-by-step GO/Next, process map,
  compact command sequence, wizard or unconstrained expert mode.
- Explicit input -> preview -> verify -> commit stages for consequential edits;
  make routine operations optionally direct and reversible.
- Clear states for pending/running/failed/completed, visible dependencies,
  actionable errors, and safe cancel/retry.
- Save checkpoints for unfinished workflows, including selection, active
  tool, parameters, preview state and a resumable task location.
- Pinned notes/checklists/diagrams and project-linked scratchpads.
- Command history, favorite operations, reusable macros, named recipes,
  templates and last-known-good parameter profiles.
- Batch actions should summarize intended changes and support dry runs.
- Optional reminders to review unsaved, unissued or unvalidated work; no
  mandatory timers, productivity scoring or attention policing.
- No forced workflow order when work can be performed safely out of sequence.

## Context-dependent workspace presets

Allow personal, project and role-based workspace presets. Examples:
- **Minimal / Focus:** canvas and a small context toolbar only.
- **Guided:** persistent step indicator, next action and validation feedback.
- **Power:** multi-pane scene/data/graph/timeline with dense inspector controls.
- **Scan & Repair:** side-by-side source/working geometry and repair checklist.
- **Rockwork:** sculpt and framing visibility, zones, dimensions and reports.
- **Model Shop:** 3D print and CNC readiness, tolerances and segmented parts.
- **Show Systems:** viewport, focus/patch sheet, cue timeline and live status
  separated clearly from design/simulation.
- **Paperwork:** layout pages, linked tables, label sheets and print preview.

Presets only affect UI and command presentation. They do not change the
underlying source geometry or data, and must not be silently written into
other collaborators' personal layouts. Presets are exportable and shareable.

## Discoverability and assistive guidance

- Searchable command palette with aliases, synonyms and examples.
- Commands explain what is selected, required inputs, expected outputs and
  whether the result can be undone.
- Helpful but optional preview overlays; guidance can be dismissed and kept
  dismissed for that workflow.
- Keyboard-accessible interfaces, screen reader labels/semantics and scalable
  text in data and document views. Support accessible non-visual feedback for
  operation status, not only colored indicators.
- Undo/redo visible for geometry, document/data edits and non-destructive
  scene changes; user-controlled autosave with clear recovery behavior.
- High precision controls remain available independent of preset simplicity.

## Data and cross-discipline consistency

Changing between CAD, mesh, metrology, procedural graph, ride/automation,
previs, database and publishing views should preserve selection and highlight
linked records, when meaningful. Project state is authoritative and
independent from the user's cognitive/workspace preferences.

Show-control state explicitly distinguishes **design**, **simulation**,
**observed telemetry** and **commanded live output**. Focus Mode must never
conceal critical armed/live-system warnings. Safety-critical feedback is
unambiguous in every visual theme and layout.

## Implementation order

1. Configuration schema for per-user UI profile, reusable workspace presets,
   autosave/recovery and shortcut maps, separate from project document.
2. Command registry aliases, command search, scope and side-effect metadata.
3. Persistent selection/tool state and predictable cross-workspace transitions.
4. Focus/Standard/Expert density layouts with color, text scale and reduced
   motion controls.
5. Guided GO/Next workflow engine with checkpoints and resumability.
6. Pinned scratchpad, checklists, progress/retry and task history.
7. Automated accessibility tests, keyboard-only workflows and long-session
   usability benchmarks across diverse voluntary testers.

## Acceptance tests

- Complete one common modeling task from minimal, guided and expert layouts
  with identical kernel geometry results.
- Switch to another workspace and back without losing active selection or
  unsaved tool parameters.
- Interrupt a multi-step scan alignment or mesh repair, reopen the project,
  and resume from a clear checkpoint.
- Operate document tables, labels and core CAD commands without a mouse.
- Increase font scale/reduce motion without obscuring dimensions or controls.
- A quiet/low-density layout still shows critical errors, conflicts and
  physical-control safety state.
- A personalized interface never changes teammates' project data or layouts.

Do not assume one workflow fits every neurodivergent person. Customizability
and reliable defaults are the goal.
