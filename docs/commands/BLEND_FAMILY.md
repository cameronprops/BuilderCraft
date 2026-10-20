# Worldwright Blend family: exact curve continuity and remaining edge/surface work

**Snapshot:** October 19, 2026. Source of truth is the code and CI on
`cameronprops/BuilderCraft`, not feature names.

Rhino 8 documentation distinguishes [Blend](https://docs.mcneel.com/rhino/8/help/en-us/commands/blend.htm),
[BlendCrv](https://docs.mcneel.com/rhino/8mac/help/en-us/commands/blendcrv.htm),
[BlendSrf](https://docs.mcneel.com/rhino/8/help/en-us/commands/blendsrf.htm),
and [BlendEdge](https://docs.mcneel.com/rhino/8/help/en-us/commands/blendedge.htm),
plus [VariableBlendSrf](https://docs.mcneel.com/rhino/8/help/en-us/commands/variableblendsrf.htm)
for radius-varying transitions between intersecting surface edges.
See [blend-parity.json](blend-parity.json) for the machine-readable inventory.

## Merged G0/G1/G2 headless curve-blend vertical slice

Merged PRs #132 and #134 share **one** geometry algorithm,
`buildercraft_kernel::blend_nurbs_curves`, and exposes it to the typed
shared operator `kernel.curve.blend`, CAD command
`worldwright.curve.blend` (alias `blend` for command lookup), and
`orbweaver.curve.blend`. It returns native `kind:"curve"` with
degree/control/weights/knots. It does **not** sample a display polyline.

All **seven typed ports** are required for the initial headless API:

| Port | Type | Semantics |
|---|---|---|
| `a`, `b` | curve | Valid rational NURBS source curves |
| `continuity` | count | **0**: position G0; **1**: geometric tangent G1; **2**: geometric curvature G2 |
| `strength_a`, `strength_b` | number | Endpoint tangent handle fractions of endpoint distance; each 0.01 to 2 |
| `reverse_a`, `reverse_b` | count | **0**: normal direction; **1**: reverse source orientation |

Default connection uses the end of A and start of B. Flipping either
orientation selects the other respective source endpoint. G0 yields a degree-one NURBS segment; G1 a degree-three polynomial
Bézier; G2 a degree-five polynomial Bézier, all encoded in the existing
rational NURBS type. The G1 source tangent directions come
from the *analytic derivatives of the original NURBS*, not sampled meshes.
The exact endpoints and source curves are retained and unmodified.
Invalid curves, close endpoints, zero cusp tangents, unsupported continuity
or strengths outside bounds fail without publishing a result.

Example envelope for `worldwright.curve.blend` or `worldwright.tool.run`:
```json
{
  "inputs": {
    "a": {"kind":"curve","value":{"degree":1,"control":[{"x":0,"y":0,"z":0},{"x":2,"y":0,"z":0}],"weights":[1,1],"knots":[0,0,1,1]}},
    "b": {"kind":"curve","value":{"degree":1,"control":[{"x":4,"y":2,"z":0},{"x":4,"y":4,"z":0}],"weights":[1,1],"knots":[0,0,1,1]}},
    "continuity": {"kind":"count","value":1},
    "strength_a": {"kind":"number","value":0.3333333333333333},
    "strength_b": {"kind":"number","value":0.3333333333333333},
    "reverse_a": {"kind":"count","value":0},
    "reverse_b": {"kind":"count","value":0}
  }
}
```

```sh
cargo test --locked -p buildercraft-kernel --lib curve_blend
cargo test --locked -p cadcraft-engine --test operational_commands
python3 tools/check_paired_tools.py
```

## G2 geometric curvature contract

At each end the quintic curve matches source positions and oriented tangent
**directions**, then chooses a normal second derivative equal to
`|B'|² * source_curvature_vector`. This is invariant to differences in
parameter speed, unlike parametric C2 matching. The control net uses the
exact endpoint Hermite formulas and the existing NURBS degree-five limit.
The same seven typed inputs apply. Different source radii and reversed
source orientations are supported, but no self-intersection avoidance,
maximum feasible blend strength, or collision certificate is yet provided.

## Explicitly **not** implemented by this slice

- No interactive Rhino-style end picking, draggable grips, trim/join,
  history dependency or persistent CAD document bake. The `blend` alias
  resolves the headless command ID but is **not** a complete interactive
  drawing tool.
- G1 aligns unit tangent directions. G2 additionally matches the source
  curves' **geometric curvature vectors** at both oriented endpoints via
  quintic Hermite interpolation, using exact rational derivatives. This
  is not parametric C1/C2 derivative magnitude matching, G3/G4 or radius
  optimization. Unsupported levels **3 and higher** explicitly reject.
- No BlendCrv two-ended curvature solver/adjustable editor yet. No BlendSrf
  tangent/curvature matching on NURBS surfaces, seam or chain-edge matching.
  No BlendEdge or VariableBlendSrf radius-handle policies, dynamic maximum
  feasible-radius feedback, or trims/joins on exact BRep topology.
- Handle fractions are not fillet or blend radii. No collision-free,
  non-self-intersecting or fabrication tolerance certificate is claimed.

## Next dependencies

1. **Merged on main:** analytic second derivatives, the shared G0/G1 solver,
   and degree-five G2 Hermite transition with independent end handle fractions.
   Numerical endpoint curvature continuity and CAD/OrbWeaver parity have
   automated tests. This is headless capability, not interactive Rhino parity.
2. Add orientation/end-pick state, preview, history and document persistence
   through normal CAD and OrbWeaver interfaces.
3. Add surface edge continuity and matched sections across untrimmed
   NURBS, then trimmed topology, chain-edge selection and variable-radius
   blend surfaces using the exact BRep backend.
4. Certify accurate continuity and failure limits, collision and trim
   diagnostics, .dftba roundtrips, undo/redo and actual viewport inputs.

PRs #132 and #134 merged into `main` on October 19, 2026. Their exact-head
native Rust, modeling and OpenCascade checks were green before #134 merged.
PR #136 was closed as redundant; its G2 core and acceptance tests overlap
with the merged solution. **Post-merge `main` checks are not independently
verified here.** Headless G0/G1/G2 is distinct from certified interactive
BlendCrv, surface blends and edge blends.
