# Worldwright Blend family: dependency and delivery status

**Snapshot:** October 19, 2026. Source of truth is the code and CI on
`cameronprops/BuilderCraft`, not feature names.

Rhino 8 documentation distinguishes [Blend](https://docs.mcneel.com/rhino/8/help/en-us/commands/blend.htm),
[BlendCrv](https://docs.mcneel.com/rhino/8mac/help/en-us/commands/blendcrv.htm),
[BlendSrf](https://docs.mcneel.com/rhino/8/help/en-us/commands/blendsrf.htm),
and [BlendEdge](https://docs.mcneel.com/rhino/8/help/en-us/commands/blendedge.htm),
plus [VariableBlendSrf](https://docs.mcneel.com/rhino/8/help/en-us/commands/variableblendsrf.htm)
for radius-varying transitions between intersecting surface edges.
See [blend-parity.json](blend-parity.json) for the machine-readable inventory.

## Proposed first operational vertical slice

The PR introduces **one** geometry algorithm,
`buildercraft_kernel::blend_nurbs_curves`, and exposes it to the typed
shared operator `kernel.curve.blend`, CAD command
`worldwright.curve.blend` (alias `blend` for command lookup), and
`orbweaver.curve.blend`. It returns native `kind:"curve"` with
degree/control/weights/knots. It does **not** sample a display polyline.

All **seven typed ports** are required for the initial headless API:

| Port | Type | Semantics |
|---|---|---|
| `a`, `b` | curve | Valid rational NURBS source curves |
| `continuity` | count | **0**: position-only G0; **1**: geometric tangent G1; **2**: geometric curvature G2 |
| `strength_a`, `strength_b` | number | Endpoint tangent handle fractions of endpoint distance; each 0.01 to 2 |
| `reverse_a`, `reverse_b` | count | **0**: normal direction; **1**: reverse source orientation |

Default connection uses the end of A and start of B. Flipping either
orientation selects the other respective source endpoint. G0 yields a
degree-one NURBS segment; G1 a degree-three polynomial Bézier, and G2 a degree-five polynomial Bézier encoded in
the existing rational NURBS type. The G1 source tangent directions come
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

## Explicitly **not** implemented by this slice

- No interactive Rhino-style end picking, draggable grips, trim/join,
  history dependency or persistent CAD document bake. The `blend` alias
  resolves the headless command ID but is **not** a complete interactive
  drawing tool.
- G1 aligns unit tangent directions; G2 also matches the geometric curvature
  vector using analytic rational endpoint derivatives and a quintic Hermite
  construction. Neither option enforces parametric C1/C2 derivative
  magnitudes, G3/G4, collision freedom, trim/joins, or radius limits.
  Unsupported levels explicitly reject.
- No BlendCrv two-ended curvature solver/adjustable editor yet. No BlendSrf
  tangent/curvature matching on NURBS surfaces, seam or chain-edge matching.
  No BlendEdge or VariableBlendSrf radius-handle policies, dynamic maximum
  feasible-radius feedback, or trims/joins on exact BRep topology.
- Handle fractions are not fillet or blend radii. No collision-free,
  non-self-intersecting or fabrication tolerance certificate is claimed.

## Next dependencies

1. Validate G2 curvature limits, non-self-intersection, and endpoint
   degeneracy diagnostics. Analytic second derivatives and a quintic G2
   Hermite blend are proposed in #133 and this stacked PR, respectively.
2. Add orientation/end-pick state, preview, history and document persistence
   through normal CAD and OrbWeaver interfaces.
3. Add surface edge continuity and matched sections across untrimmed
   NURBS, then trimmed topology, chain-edge selection and variable-radius
   blend surfaces using the exact BRep backend.
4. Certify accurate continuity and failure limits, collision and trim
   diagnostics, .dftba roundtrips, undo/redo and actual viewport inputs.

This status is only a proposed PR until its pinned formatting, native Rust,
modeling acceptance and OpenCascade integration checks pass and it merges
into `main`.
