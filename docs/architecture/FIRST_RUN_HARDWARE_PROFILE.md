# First-launch hardware profile and performance routing

WorldWright desktop automatically inspects host capabilities on first launch
(the first run after installation) and saves a small **versioned hardware
profile** under eframe's existing persistent app storage key
`worldwright.hardware.v1`. On subsequent launches it compares the OS/arch,
available CPU parallelism, usable system RAM and **capabilities of the actual
wgpu device** with the stored fingerprint. The profile is reused when
unchanged and retuned when hardware, drivers/capabilities or schema change.
The user-selected Auto/Economy/Balanced/Performance mode survives reprofile.

Hardware detection uses only std and the already required wgpu runtime.
Linux reads MemTotal and honors smaller cgroup v2 memory.max when present;
macOS queries hw.memsize once; Windows requests TotalPhysicalMemory through
PowerShell/CIM once at startup; unsupported hosts use safe unknown-memory
defaults. None of these diagnostics is sent to the network. This does NOT
measure actual GPU VRAM, GPU model, thermal headroom or benchmark throughput.

## Applied tuning in this vertical slice

- CPU topology, B-rep, mesh repair, exact geometry, and current CPU-painted
  3D viewport always stay on CPU for deterministic correctness.
- The already operational **2D wgpu display-list renderer** is chosen when
  the real device exists, power mode permits it and the previous frame's
  display-list primitive count exceeds the profile threshold. Small drawings
  remain on the CPU to avoid upload overhead; Economy forces CPU.
- Interactive mesh drawing and CPU mesh picking receive the **same** adaptive
  face budget, capped by the current 15,000-face interactive limit.
  Limited-RAM hosts use smaller limits, preventing invisible pick targets.
- Stream RAM budget and worker limit are stored as advisory targets for
  forthcoming scan/VDB/parallel numerical operations; **the current Rayon
  global pool is not modified** and no unimplemented GPU compute backend is
  falsely treated as operational.

Under **Performance** in both CAD and Mesh Repair, select Auto (default),
Economy, Balanced or Performance, or choose **Reprofile hardware**. Profiles
are independent of .dftba documents and undo/redo.

## Missing prerequisites and acceptance

To accelerate hole fill, propagated triangles, scan alignment, deviation maps
and voxel/SDF operations on GPU, implement, prove and benchmark a dedicated
compute pipeline first. Schedule GPU only when kernels exist, fit within
reported resource limits, and pass CPU/GPU equivalence tests. Exact B-rep
geometry remains on CPU. Add workload-specific benchmarks and rendering
telemetry before automatic GPU repair dispatch. No speculative vendor-based
GPU speed classes are assigned.

Tests cover CPU fallback, low-memory budget, actual GPU 2D threshold,
manual mode, versioned profile reuse/migration and consistent pick/draw limits.
CI across Linux/macOS/Windows is required.
