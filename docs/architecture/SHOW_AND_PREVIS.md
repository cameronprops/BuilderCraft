# Show equipment, paperwork and previs

Status: target contract; not implemented in alpha 0.1. BuilderCraft Show is independently usable and connects to the shared scene through stable IDs.

## Equipment records

Every unit has a UUID, profile/model reference and revision, category, display name, unit number, position/location, transform or optional scene object, manufacturer/model/mode, purpose, group/system, notes and source. Fixtures also have console fixture ID/channel, universe, start address, footprint, focus, accessories, circuit/dimmer, power/load and cable/network relationships. Multiple patch breaks, multipatch and mode-specific footprints are modeled explicitly; no assumption that every device occupies one contiguous range.

Categories include lighting fixtures, dimmers, practicals, speakers/amplifiers, microphones, projectors, video screens, media servers, sensors, IO, actuators, motors, winches and moving scenery. Physical equipment identity is separate from console numbers, channels, universe/address or cue IDs, all of which can change. Units can be grouped by system, position, zone, height, layer, body or user criteria without duplicating the record.

Mechanical units add axes, joints, home/neutral state, travel limits, units, speed/acceleration metadata and path references. AV units add signal flow, connectors, transport, destinations and routing. A schematic or equipment schedule can exist without a 3D asset. Unknown physical data remains explicit.

Reports: instrument schedule, channel/fixture hookup, patch and universe utilization, position lists, equipment/accessory counts, cable and network schedules, circuit/power summaries, automation axis/IO lists and revision/change reports. Filters and templates use the same underlying records as the scene and patch. Export CSV first; formatted sheets and PDFs follow.

## Patch and cue validation

Patch validator checks address ranges and mode footprints, collisions, duplicate IDs, universe conventions, split breaks, invalid fixture references, unavailable modes and explicit multipatch policy. No silent renumbering. Imports retain external identifiers and their mapping.

The internal cue model stores intent in semantic/physical attributes: intensity, color, focus target, pan/tilt, beam, media choice and kinematic pose/time. It also records cue IDs, labels, timing, parts, follow/link/triggers, tracking policy and interpolation. Fixture profiles map this to control values. DMX values alone cannot carry every console's cue semantics.

Normalize patch separately from cue intent and manufacturer-specific programming. Every console adapter declares tested product/software version, supported patch and cue features, profile matching rules and omissions. A successful file write does not establish successful console import.

## Interchange strategy

| Target | Planned role | Acceptance requirement |
|---|---|---|
| GDTF | Fixture geometry, attributes, DMX modes and physical/profile information | Parse/serialize declared version, mode/profile matching and attribution preservation |
| MVR | Rig and scene hierarchy, device placement and logical patch exchange | Roundtrip IDs, transforms, patch and embedded resources; retain unsupported untouched fields where permitted |
| CSV/JSON | Neutral equipment, reports, patch and cue-intent exports | Versioned schema, units, stable IDs and loss report |
| ETC Eos | Target-specific patch and pre-cue adapter through supported documented import paths | Test against a specific Eos version; use its exported template/schema for CSV; test generic ASCII limitations explicitly |
| MA grandMA3 | MVR patch/scene route; separate cue-programming route | Tested MVR import and profile matching; separate version-specific cue translator |
| Other console families | Adapter backlog: grandMA2, Hog, Avolites, ChamSys, Onyx, ETC Cobalt and others requested | Investigate supported format/API and verify patch and cue support individually |

All major console families are a product goal. Until a console/version passes roundtrip/import fixtures, mark it unimplemented or experimental. MVR and GDTF are not universal cue/showfile interchange. Basic USITT ASCII cues do not imply moving-light, effect or macro parity. Console-specific exports must explain unsupported tracking, effects, palettes, timing, macros and profile matching instead of dropping them silently.

Planning a DMX network and sending live DMX are separate features. Previs first reads simulated values/cues. Future Art-Net, sACN, OSC, MIDI/timecode and device control adapters are explicit capabilities with physical output disabled by default. Simulation export does not automatically commission real motion-control hardware.

## Previs pipeline

Shared scene + equipment model + cue timeline produces simulation input. The same unit UUID connects CAD placement, report row, patch, simulated fixture and engine actor. Show can preview lightweight placeholders without CAD; engine export uses richer assets when available. Reimport updates preserve manual engine content by maintaining imported and user-authored ownership boundaries.

First walkthrough gate: a massing model and one fixture become a navigable Unreal scene at the expected scale; a second export updates that same object rather than duplicating it. Follow with spot/intensity/color preview, a camera path, one kinematic moving element and a cue timeline. Later include photometry, beam/gobo interpretation, projection mapping, spatial audio, effect particles and coordinated show timing.

## Sources and version checks

- GDTF developer specifications: https://gdtf-share.com/help/developers/
- MVR 1.6: https://gdtf-share.com/help/developers/mvr_1_6/index.html
- Eos import: https://www.etcconnect.com/WebDocs/Controls/EosFamilyOnlineHelp/en/Content/05_Show_Files/Importing_Show_Data.htm
- Eos CSV guidance: https://support.etcconnect.com/ETC/Consoles/Eos_Family/Software_and_Programming/Importing_and_Exporting_CSV_Files_for_Eos
- Eos USITT ASCII limits: https://support.etcconnect.com/ETC/Consoles/Eos_Family/Software_and_Programming/Eos_Family_USITT_ASCII_Import
- grandMA3 2.4 MVR: https://help.malighting.com/grandMA3/2.4/HTML/patch_mvr.html
- Unreal glTF: https://dev.epicgames.com/documentation/en-us/unreal-engine/gltf-file-format-support-in-unreal-engine

References reviewed 2026-10-07. Actual adapters pin and test a supported version; these links are research, not implemented compatibility.
