# StructureGraph reuse register

Source: https://github.com/cameronprops/StructureGraph . Reviewed 2026-10-07 from actual README, CURRENT_STATE, ARCHITECTURE, INVARIANTS, module registry, rail contracts and Trylon/Perisphere geometry README. No StructureGraph source has been copied or runtime-integrated by this decision commit.

| Existing work | Verified maturity | BuilderCraft reuse |
|---|---|---|
| Serializable recipes and guide/preview/final/QA stages | Trylon/Perisphere extracted Python stages and Rhino adapter spike; real Rhino validation pending in source state | Graph recipes, explicit job stages, units and stable generated ownership; first reference case |
| Geometry/output and fabrication validation contracts | Contract spikes | Shared output modes, tolerance and transactional validation contract; closed-solid preservation |
| MakerCore rail/frame/sweep contracts | C# contracts referencing Rhino types | Backend-neutral rail station/frame recipe, deterministic frames, sweep and repeated assemblies; optional Rhino implementation |
| BasketWeaver braid/weave and fabrication behavior | Legacy working sources reported; not independently executed in this review | Recover exact source and fixtures; then wrap or extract validated pure math for Graph/CAD |
| RockForge fields/noise and panel concepts | Experimental/early procedural alpha | Procedural scenic surfaces, mesh/volume intermediates, panel boundaries and fabrication QA |
| SpliceConnector | Contract/preview spike; connector cutters remain future work | Connector semantics and preserve-source validation, not a claim that connectors are complete |
| Provenance and truth schemas | Design established and case-study scaffolds | Sources, evidence, dimensions and confidence carried across apps and engine exports |
| Kangaroo orchestration | Rhino-dependent contract spike | Optional Rhino worker/backend; do not require it for independent BuilderCraft or copy a solver blindly |
| Track/ride-path, gears/springs, player piano causal systems | Mostly planned | Shared simulation/kinematic requirements and regression scenes as implementation lands |

## Extraction procedure

1. Locate exact implementation and fixtures; distinguish executable code from contracts/plans.
2. Preserve legacy revision, attribution and applicable license. Review source dependencies before copying or packaging; do not assume source ownership overrides third-party terms.
3. Separate pure recipe/data/math from RhinoCommon/Kangaroo calls. Prefer a bridge around host-dependent operations until a native backend is verified.
4. Add backend-neutral contracts using stable IDs, units, tolerances, provenance, output kinds and quality.
5. Compare new results with deterministic fixtures and invariants before promoting to shared core.
6. Register one canonical shared operation used by direct commands, graph nodes and APIs.

StructureGraph remains a valid Rhino plugin platform. BuilderCraft's new independent CAD goal does not silently replace that project's Rhino-authoritative architecture. The same portable recipes may target both backends, with capability and representation differences reported explicitly.
