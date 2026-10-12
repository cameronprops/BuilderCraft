# FreeCAD-derived dependency traversal

A small Rust library adapted from FreeCAD's **actual** C++ document dependency
walker, not a replacement for Worldwright's kernel, parametric model, or BRep
engine.

Upstream:
- Project: https://github.com/FreeCAD/FreeCAD
- Revision: `ec6852e6caadf55045881a6003cc7ad038c8d2a7`
- File: `src/App/DocumentObject.cpp`
- Method: `DocumentObject::getInListEx`
- Upstream copyright: 2011 Jürgen Riegel and Werner Mayer
- SPDX: `LGPL-2.1-or-later`
- Adaptation date: 2026-10-11

Worldwright changes: numeric IDs instead of pointers, validated
`(prerequisite, dependent)` edges, stack-based traversal, stable sorted output,
strict edge/node admission budgets, fallible capacity reservations, and
independent unit tests. It is called from Worldwright's shared history
implementation and therefore usable from CAD/API without a second algorithm.

The separate crate deliberately isolates FreeCAD-derived LGPL code from
Worldwright's pre-existing MIT/Apache source. Distributors must comply with
LGPL conditions, include the license and notices, and provide the required
source/relinkability arrangements for any linked binary. This is **not** the
whole FreeCAD runtime and does not automatically import FreeCAD's Sketcher,
PartDesign or TopoShape functionality.
