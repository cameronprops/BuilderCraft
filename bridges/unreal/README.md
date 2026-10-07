# BuilderCraft Unreal visualization feed

Status: native publication/export tests pass; the original Unreal adapter source is included but has NOT been compiled or exercised in Unreal here. An Unreal SDK/host is unavailable in this environment. Target UE 5.5+ APIs; verify the selected host version before relying on it. This directory is dual MIT/Apache-2.0 under the repository licenses; it contains original adapter code and no engine source. Unreal remains an independently installed, optional visualization backend.

## Start the local feed

Save a `.bcraft` project with supported units and exact 3D geometry. Then run:

```sh
cargo run -p cadcraft-cli -- visualize-watch /absolute/model.bcraft /absolute/feed 00000000000000000000000000000001
```

The watcher checks saved file changes every 0.5 seconds and publishes complete validated snapshots. Add `--once` for a single publication. The saved-project route updates after saves, not after every unsaved drag. From a running native CAD session, invoke `visualization.publish` through the existing local API with `{ "project_id":"00000000000000000000000000000001", "directory":"/absolute/feed" }`; it publishes the current unsaved document without changing its revision or save path. Automatic debounced publication of every in-memory CAD edit is the next live-feed gate.

One directory per project, one writer. `snapshot.json` is the commit marker; uniquely sequenced GLB files are written first, then the JSON snapshot is atomically replaced. Sequence survives producer restarts; source revision is a string (CAD revision, or saved-file fingerprint in watch mode). Two recent GLBs are retained. A crashed writer can leave `writer.lock`; remove it only after confirming that writer stopped. No sockets, cloud upload or remote code execution.

## Unreal setup and host acceptance

1. Copy `BuilderCraftBridge` into the Unreal project's `Plugins` directory, regenerate project files and compile the project/plugin. Enable the plugin and ProceduralMeshComponent dependency.
2. Place a `BuilderCraftSceneFeed` actor at the world origin. Set SnapshotPath to the absolute `snapshot.json` path and ExpectedProjectId to the chosen project ID. Assign a two-sided massing material for open control surfaces.
3. Use Refresh in the editor; enable realtime viewport ticking for polling. Play mode polls as well. Use an existing first-person/walkthrough pawn. Enable CreateCollision only when needed; collision cooking/performance must be tested on the host.
4. Test create, rename, edit, visibility, reparent, delete and delete-all. Repeated snapshots must keep the same object/component IDs. Stale sequences and other project IDs must be rejected. A malformed snapshot must preserve the last valid scene.
5. Verify the included 2000 x 3000 mm plane becomes 200 x 300 Unreal centimetres with correct axes, winding and visibility. Test restart, rapid saves and a disconnected/reconnected feed. Check materials, collisions and component/GPU teardown.

The adapter validates file size, JSON depth, version, project, sequence, geometry bounds/index counts and parent cycles before mutating components. It applies complete snapshots by stable ID, caches geometry keys, removes deleted components and retains production metadata in ProductionJson. Default collision is off. The first live renderer displays triangulated surfaces; curve polylines are in portable GLB but not yet rendered by the runtime adapter. Production relationships currently travel as metadata, not animated show/device logic.

## Coordinates and limitations

Snapshot coordinates: left-handed Z-up **metres**. The adapter multiplies positions by 100 for Unreal centimetres. Portable GLB: standard right-handed Y-up metres; export handles axis conversion and winding separately. Source IDs/hierarchy/visibility/production references are preserved. Geometry uses absolute drawing coordinates beneath identity hierarchy transforms.

Preview surfaces are uniform untrimmed samples, not robust solids. No calibrated lighting/AV, original materials, instancing, textures, trims, ride motion, envelope/sightline analysis or full scene delta transport yet. Limits: 256 objects, 50,000 vertices, 100,000 triangles, 32 MiB packages, 8 MiB watch input. Work is bounded but native command export and host parsing are synchronous; async/coalesced jobs and measured RSS/GPU performance are release gates. Fingerprints are non-security cache hints, not authentication or cryptographic provenance. GLB validation here uses the independent MIT/Apache-2.0 `gltf` parser; full Khronos and Unreal host acceptance remain separate.

References: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html and https://dev.epicgames.com/documentation/en-us/unreal-engine/coordinate-system-and-spaces-in-unreal-engine .
