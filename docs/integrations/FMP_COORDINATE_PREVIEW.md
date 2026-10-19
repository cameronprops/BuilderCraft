# Flushing Meadows Project bridge (Worldwright)

Status: experimental preview interface, not certified site alignment or complete BIM import. The FMP bridge is maintained in the companion repository:
https://github.com/cameronprops/TheFlushingMeadowProject/tree/main/tools/georeferencing/importers

## Architecture: reuse native code
Worldwright is the product name of this repository (BuilderCraft is still the repository/crate name). The FMP adapter deliberately uses existing CAD IO:
- crates/io/src/dxf_read.rs parses source-layer POINT, LINE and TEXT entities.
- crates/io/src/project.rs serializes the native Worldwright .dftba project.
- apps/cadcraft-cli/src/main.rs provides info and convert commands.
Do not create a second DXF parser or write an ad hoc .dftba JSON file.

## Working handoff
1. Generate synthetic or source-verified placements in FMP from fmp.georef.scene.v1 and fmp.markers.v1; source-map coordinates are transformed into a *single local site frame*.
2. Run FMP Python exporter fmp_export_worldwright.py to produce a plain ASCII AC1015 DXF and external audit JSON.
3. Header declares INSUNITS=6 (metres), MEASUREMENT=1. Every source-map layer is named FMP_<map-id>. POINT is the location; 1m crosshair LINE entities and TEXT ID provide viewport identification. Z=0 is an annotation plane.
4. Open a **new** drawing in Worldwright or convert on the CLI, using the native file readers/writers:

    cargo run -p cadcraft-cli -- info /absolute/fmp-preview.dxf
    cargo run -p cadcraft-cli -- convert /absolute/fmp-preview.dxf /absolute/fmp-preview.dftba

5. Keep the accompanying .fmp.json sidecar with the project as provenance (source IDs, normalized metre positions, data class and DXF SHA-256); the sidecar is not currently embedded in the .dftba schema.

## Coordinate discipline
- Canonical geodetic basis in the FMP manifest: NAD83 New York Long Island projected CRS EPSG:32118; metre-based.
- Site-local X points east; Y points north; Z datum is unresolved. Do not confuse source pixels with CAD units.
- The DXF stores **local metre** positions; inspect resulting .dftba header INSUNITS and confirm entity positions are unchanged.
- Rhino 8 converts metre positions into current document units. Unreal Editor converts metres to centimetres. All three receive the same placement list from FMP.
- The modern coordinate origin must be surveyed and documented; available sample origin and points are synthetic only.
- No automatic reprojection, elevation, terrain reconstruction, point synchronization or real-world survey certification is implied.

## Acceptance gates
- Python-side mocked integration tests and independent DXF format audit have passed in the FMP environment.
- Run a **native** Rust round trip on the development host; confirm modelspace entity count = 16, POINT count = 4, LINE count = 8, TEXT count = 4, coordinates preserved to model tolerance, units still metres, and independent sidecar SHA matches.
- Test Save/Open/Save .dftba and view all annotations through the actual desktop UI; no screen/host validation has been completed by this integration work.
- Validate an actual multi-era historical dataset only after acquiring and checking period drawings and independent survey control points.
- Do not treat this simple annotated DXF exchange as general historical geometry, an entire 1939/1964 world, or Worldwright command parity.

## Next native Worldwright milestone
Provide an optional Worldwright source-map import command with stable marker IDs, reference-link metadata and update-in-place semantics, using shared project/document core services. Reuse the same FMP schema and converter, rather than adding another independent survey engine. Test with Rust/Cargo and UI before advertising an operational native importer.

Historical observations and fictional 2039/2064 branches must stay in separate evidence and world-state namespaces.
