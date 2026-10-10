//! Portable first-launch hardware profiling and truthful CPU/GPU routing.
//! No extra runtime dependencies, no blocking benchmark, no invented VRAM.
//! Geometry and topology remain CPU tasks until a tested GPU compute backend exists.
use egui_wgpu::wgpu;
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;
const MIB: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerMode {
    Auto,
    Economy,
    Balanced,
    Performance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Graphics {
    /// A wgpu device was created successfully for this session.
    pub canvas_device: bool,
    pub max_texture_2d: u32,
    pub max_storage_binding_bytes: u32,
    /// Device reports compute limits; this does NOT mean GPU mesh repair exists.
    pub compute_capable: bool,
}

impl Graphics {
    pub fn unavailable() -> Self {
        Self { canvas_device: false, max_texture_2d: 0, max_storage_binding_bytes: 0, compute_capable: false }
    }

    pub fn probe(render: Option<&egui_wgpu::RenderState>) -> Self {
        let Some(render) = render else { return Self::unavailable() };
        let limits = render.device.limits();
        Self {
            canvas_device: true,
            max_texture_2d: limits.max_texture_dimension_2d,
            max_storage_binding_bytes: limits.max_storage_buffer_binding_size,
            compute_capable: limits.max_compute_workgroups_per_dimension > 0 && limits.max_compute_workgroup_size_x > 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hardware {
    pub os: String,
    pub arch: String,
    pub available_cpu_threads: usize,
    /// OS-reported physical memory (or the smaller container memory limit).
    /// Unknown is not treated as zero, or guessed to be a huge amount.
    pub usable_memory_bytes: Option<u64>,
    pub graphics: Graphics,
}

impl Hardware {
    pub fn detect(render: Option<&egui_wgpu::RenderState>) -> Self {
        Self {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            available_cpu_threads: std::thread::available_parallelism().map_or(1, usize::from),
            usable_memory_bytes: usable_memory(),
            graphics: Graphics::probe(render),
        }
    }
}

fn parse_first_integer(output: &[u8]) -> Option<u64> {
    String::from_utf8_lossy(output).split_whitespace().find_map(|piece| piece.trim().parse::<u64>().ok()).filter(|n| *n > 0)
}

#[cfg(target_os = "linux")]
fn usable_memory() -> Option<u64> {
    let report = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kib = report.lines().find_map(|line| {
        line.strip_prefix("MemTotal:").and_then(|rest| rest.split_whitespace().next()).and_then(|n| n.parse::<u64>().ok())
    })?;
    let physical = kib.checked_mul(1024)?;
    let cgroup = std::fs::read_to_string("/sys/fs/cgroup/memory.max")
        .ok()
        .and_then(|line| line.trim().parse::<u64>().ok())
        .filter(|n| *n > 0);
    Some(cgroup.map_or(physical, |limit| physical.min(limit)))
}

#[cfg(target_os = "macos")]
fn usable_memory() -> Option<u64> {
    let result = std::process::Command::new("/usr/sbin/sysctl").args(["-n", "hw.memsize"]).output().ok()?;
    result.status.success().then(|| parse_first_integer(&result.stdout)).flatten()
}

#[cfg(target_os = "windows")]
fn usable_memory() -> Option<u64> {
    // One-time native setup probe only. A missing PowerShell/locked-down host
    // means unknown RAM and conservative settings, not a failed launch.
    let result = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory"])
        .output()
        .ok()?;
    result.status.success().then(|| parse_first_integer(&result.stdout)).flatten()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn usable_memory() -> Option<u64> {
    // BSD/Haiku and other targets fall back to safe small budgets.
    None
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tuning {
    /// Advisory bound for future bounded worker pools; this module does not
    /// silently mutate Rayon global thread pools already used by the kernel.
    pub cpu_worker_limit: usize,
    /// Interactive polygon preview and picking share this exact face limit.
    pub viewport_faces: usize,
    /// Conservative target for streaming/chunked scan algorithms (not pre-allocated).
    pub streaming_budget_bytes: u64,
    /// Min 2D display primitives before the already-implemented wgpu canvas
    /// path wins in Auto mode. Other modeling kernels stay on CPU.
    pub gpu_canvas_min_primitives: usize,
}

impl Tuning {
    pub fn for_machine(machine: &Hardware, mode: PowerMode) -> Self {
        let cpus = machine.available_cpu_threads.max(1);
        let reserve = if cpus >= 4 { 1 } else { 0 };
        let workers = cpus.saturating_sub(reserve).clamp(1, 32);
        let memory = machine.usable_memory_bytes.unwrap_or(1024 * MIB);
        let memory = memory.clamp(256 * MIB, 256 * 1024 * MIB);
        let target = (memory / 10).clamp(32 * MIB, 2048 * MIB);
        let faces = if memory < 2 * 1024 * MIB {
            3_000
        } else if memory < 6 * 1024 * MIB {
            7_500
        } else {
            15_000
        };
        let (worker_scale, faces_scale, budget_scale, gpu_threshold) = match mode {
            PowerMode::Economy => (1, 2, 2, usize::MAX),
            PowerMode::Balanced => (1, 1, 1, 1_500),
            PowerMode::Performance => (1, 1, 1, 384),
            PowerMode::Auto => (1, 1, 1, if memory < 2 * 1024 * MIB { 3_000 } else { 800 }),
        };
        Self {
            cpu_worker_limit: (workers / worker_scale).max(1),
            viewport_faces: (faces / faces_scale).clamp(1_000, crate::mesh_picking::MAX_VIEWPORT_FACES),
            streaming_budget_bytes: (target / budget_scale).max(16 * MIB),
            gpu_canvas_min_primitives: gpu_threshold,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub schema_version: u32,
    pub hardware: Hardware,
    pub mode: PowerMode,
    pub tuning: Tuning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Canvas2d { primitives: usize },
    ExactGeometry,
    MeshTopology,
    MeshRepair,
    MeshViewport3d,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compute {
    Cpu,
    Gpu,
}

impl Profile {
    pub fn safe_default() -> Self {
        let hardware = Hardware {
            os: std::env::consts::OS.into(), arch: std::env::consts::ARCH.into(),
            available_cpu_threads: 1, usable_memory_bytes: None, graphics: Graphics::unavailable(),
        };
        let tuning = Tuning::for_machine(&hardware, PowerMode::Auto);
        Self { schema_version: SCHEMA_VERSION, hardware, mode: PowerMode::Auto, tuning }
    }

    /// Compare a tiny, persisted device signature every launch. Reuse tuned
    /// values if unchanged; retune after OS/GPU/memory changes or a schema bump.
    /// Explicit user-selected power mode survives a hardware reprofile.
    pub fn initialize(detected: Hardware, stored: Option<&str>) -> Self {
        let previous = stored.and_then(|json| serde_json::from_str::<Self>(json).ok());
        if let Some(old) = &previous
            && old.schema_version == SCHEMA_VERSION
            && old.hardware == detected
            && old.tuning == Tuning::for_machine(&detected, old.mode)
        {
            return old.clone();
        }
        let mode = previous.filter(|p| p.schema_version == SCHEMA_VERSION).map_or(PowerMode::Auto, |p| p.mode);
        Self { schema_version: SCHEMA_VERSION, tuning: Tuning::for_machine(&detected, mode), hardware: detected, mode }
    }

    pub fn mode(&mut self, mode: PowerMode) {
        self.mode = mode;
        self.tuning = Tuning::for_machine(&self.hardware, mode);
    }

    /// GPU dispatch is enabled ONLY for the existing 2D wgpu canvas.
    /// Exact NURBS/B-rep, Boolean, mesh repair, inspection and the 3D painter
    /// stay CPU-bound until individually proven equivalent GPU implementations
    /// exist. Never imply a device supports a non-existent compute pipeline.
    pub fn route(&self, operation: Operation) -> Compute {
        if let Operation::Canvas2d { primitives } = operation
            && self.hardware.graphics.canvas_device
            && primitives >= self.tuning.gpu_canvas_min_primitives
        {
            Compute::Gpu
        } else {
            Compute::Cpu
        }
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Shared compact UI for CAD and Mesh Repair; mode changes never change
/// geometry, document revisions or binary save format.
pub fn menu(app: &mut crate::CadApp, ui: &mut egui::Ui) {
    ui.menu_button("Performance", |ui| {
        let hw = &app.machine_profile.hardware;
        ui.strong(format!("{} threads detected", hw.available_cpu_threads));
        if let Some(memory) = hw.usable_memory_bytes {
            ui.weak(format!("Usable RAM: {} MiB", memory / MIB));
        } else {
            ui.weak("Memory size unknown (conservative profile)");
        }
        ui.weak(if hw.graphics.canvas_device { "GPU canvas available" } else { "CPU canvas fallback" });
        ui.separator();
        for (label, mode) in [
            ("Auto", PowerMode::Auto), ("Economy", PowerMode::Economy),
            ("Balanced", PowerMode::Balanced), ("Performance", PowerMode::Performance),
        ] {
            if ui.selectable_label(app.machine_profile.mode == mode, label).clicked() {
                app.set_power_mode(mode);
                ui.close();
            }
        }
        ui.separator();
        if ui.button("Reprofile hardware").clicked() {
            app.reprofile_machine();
            ui.close();
        }
        ui.weak(format!("{} preview faces · {} MiB stream cap",
            app.machine_profile.tuning.viewport_faces, app.machine_profile.tuning.streaming_budget_bytes / MIB));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(cpus: usize, mem_gib: u64, gpu: bool) -> Hardware {
        Hardware {
            os: "test-os".into(), arch: "test-arch".into(), available_cpu_threads: cpus,
            usable_memory_bytes: Some(mem_gib * 1024 * MIB),
            graphics: if gpu {
                Graphics { canvas_device: true, max_texture_2d: 8192, max_storage_binding_bytes: 65536, compute_capable: true }
            } else { Graphics::unavailable() },
        }
    }

    #[test]
    fn first_launch_chooses_safe_cpu_budget_and_graphics_path() {
        let cpu = Profile::initialize(hw(2, 1, false), None);
        assert_eq!(cpu.mode, PowerMode::Auto);
        assert_eq!(cpu.tuning.viewport_faces, 3_000);
        assert_eq!(cpu.route(Operation::Canvas2d { primitives: 50_000 }), Compute::Cpu);
        assert_eq!(cpu.route(Operation::MeshRepair), Compute::Cpu);
        assert_eq!(cpu.route(Operation::ExactGeometry), Compute::Cpu);
        assert_eq!(cpu.route(Operation::MeshViewport3d), Compute::Cpu);
    }

    #[test]
    fn mode_overrides_and_gpu_canvas_threshold_are_applied() {
        let mut profile = Profile::initialize(hw(12, 32, true), None);
        assert_eq!(profile.tuning.viewport_faces, crate::mesh_picking::MAX_VIEWPORT_FACES);
        assert_eq!(profile.route(Operation::Canvas2d { primitives: 799 }), Compute::Cpu);
        assert_eq!(profile.route(Operation::Canvas2d { primitives: 800 }), Compute::Gpu);
        assert_eq!(profile.route(Operation::MeshTopology), Compute::Cpu);
        profile.mode(PowerMode::Economy);
        assert_eq!(profile.route(Operation::Canvas2d { primitives: 1_000_000 }), Compute::Cpu);
        assert!(profile.tuning.viewport_faces < 15_000);
        profile.mode(PowerMode::Performance);
        assert_eq!(profile.route(Operation::Canvas2d { primitives: 384 }), Compute::Gpu);
    }

    #[test]
    fn cached_first_run_profile_is_reused_and_retuned_on_hardware_change() {
        let mut initial = Profile::initialize(hw(4, 8, true), None);
        initial.mode(PowerMode::Balanced);
        let serialized = initial.json();
        assert_eq!(Profile::initialize(hw(4, 8, true), Some(&serialized)), initial);
        let moved = Profile::initialize(hw(8, 16, false), Some(&serialized));
        assert_eq!(moved.mode, PowerMode::Balanced);
        assert_ne!(moved.tuning, initial.tuning);
        assert_eq!(moved.route(Operation::Canvas2d { primitives: 9_999 }), Compute::Cpu);
        let bad = Profile::initialize(hw(8, 16, false), Some("{broken"));
        assert_eq!(bad.mode, PowerMode::Auto);
    }

    #[test]
    fn corrupted_or_outdated_tuning_cannot_force_unsafe_preview_budget() {
        let hw = hw(8, 4, true);
        let mut old = Profile::initialize(hw.clone(), None);
        old.tuning.viewport_faces = usize::MAX;
        let repaired = Profile::initialize(hw, Some(&old.json()));
        assert!(repaired.tuning.viewport_faces <= crate::mesh_picking::MAX_VIEWPORT_FACES);
        assert!(repaired.tuning.streaming_budget_bytes >= 16 * MIB);
        assert_eq!(parse_first_integer(b" 17179869184 \n"), Some(17_179_869_184));
        assert_eq!(parse_first_integer(b"unknown"), None);
    }
}
