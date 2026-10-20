# Wolfram mathematical oracle for the native Blend family

**Scope:** four checked-in independent analytic fixtures for the shared Rust
`buildercraft_kernel::blend_nurbs_curves` implementation, G0/G1/G2.
This is verification work, **not** a replacement CAD engine, standalone Wolfram
integration, or complete command certification.

## Files and procedure

- `tools/wolfram/blend_oracle.wl`: readable Wolfram Language generator.
- `crates/kernel/tests/fixtures/wolfram_blend_oracle.json`: pinned numeric oracle.
- `crates/kernel/tests/wolfram_blend_oracle.rs`: standalone Rust integration test.
- Run in the ordinary Rust workspace, with **no Wolfram installation required**:
  `cargo test --locked -p buildercraft-kernel --test wolfram_blend_oracle`.
- To reproduce the reference data when Wolfram Language is available:
  `wolframscript -file tools/wolfram/blend_oracle.wl > crates/kernel/tests/fixtures/wolfram_blend_oracle.json`.
  Compare numeric JSON values after regeneration. Regenerating must not be a
  required CI step or a prerequisite for using the open-source application.

The generator starts from integer/rational source points, rational parameters,
and the exact algebraic quarter-circle weight `1/Sqrt[2]`. Wolfram performs
analytic rational Bézier evaluation/differentiation and quintic Hermite
construction, then exports 16-digit numeric references. Source curves are
**degree-one or degree-two** rational Béziers, so this corpus tests exact
mathematics within the shared kernel's NURBS family, not arbitrary knot
multiplicities or topology.

## Verified invariants and limitations

For each of four independent cases (G0 spatial segment; G1 spatial tangent
connection; G2 polynomial quadratic rails; G2 rational circular rail with
reversed second endpoint), compare the exact native degree, all control
points, five curve points and five first derivatives to Wolfram-generated
reference values. For both G2 cases, also compare the geometric curvature
vector at both endpoints **and at the midpoint**, a stronger check than
endpoint-only continuity. The Rust test verifies source curves are unchanged.
Tolerances are currently per-vector `2e-8 * max(1, |expected|)`: they are a
specific baseline regression criterion, **not** a physical fabrication
tolerance or a universally scale-invariant acceptance policy.

No support or certification is implied for BlendSrf, BlendEdge, VariableBlendSrf,
STEP, trimmed NURBS, self-intersection-free blends, valid topology, GUI input,
`.dftba` persistence, CAD/OrbWeaver command dispatch, or sweep clearance.
Those require distinct release gates. The current kernel's G2 contract is
**geometric curvature** rather than parametric C2.

## Wolfram-identified known limitation: interior stationary points

The G1 blend currently permits handle strengths `strength_a=2`,
`strength_b=2`. For collinear source rails whose oriented endpoint
tangents point in opposing directions, the resulting cubic can be

```text
P0=(1,0,0)  P1=(2,0,0)  P2=(2.5,0,0)  P3=(1.5,0,0)
```

Wolfram's **exact** derivative for its x-coordinate is
`3 (1 - t - t²)`. The interior zero-speed parameter is
`(sqrt(5)-1)/2 = 0.6180339887498948...`. The curve reaches
`x=(-3+5 sqrt(5))/4 = 2.045084971874737...` before reversing,
overshooting the second endpoint x=1.5 by `0.545084971874737...`.
G1 endpoint tangency does **not** prove regular interior geometry.

**Next shared-kernel task:** add a bounded deterministic preflight diagnostic
for interior speed degeneracy, curvature spikes, self-overlap and intersections,
and add tests proving failure leaves documents unchanged. Do not blindly
reduce allowed user handle settings or silently replace shapes without
explicit modifier/diagnostic semantics. Since even a sampled curvature check
does not certify global self-intersection freedom, use analytic polynomial
tests where possible and label other results as advisory. Integrate the
same diagnostic with CAD, OrbWeaver, UI previews and API once proven.

## Maintenance

Treat the versioned reference fixture as source-controlled test evidence.
Review changes in source equations, floating-point tolerance and generator
revision together. Capture the pinned Rust cross-platform CI conclusions
before promoting any certification ledger status. New oracle families should
extend existing shared-kernel tests without introducing a second runtime
geometry authority or a Wolfram licensing dependency.
