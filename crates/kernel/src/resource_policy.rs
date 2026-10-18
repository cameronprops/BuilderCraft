//! Portable process-local resource admission for the Worldwright suite.
//!
//! Operating-system probes provide *advisory* memory samples. This module
//! does not query platform APIs, allocate actual memory or guarantee that an
//! operating-system allocation will succeed. Clients must reserve before
//! allocating, use fallible allocation where possible, and release leases.
//! GPU capacity is separate because it may be dedicated or shared physical RAM.
use crate::{KernelError, Result};
use std::sync::{Arc, Mutex, MutexGuard};

const MIB: usize = 1024 * 1024;
const MIN_BUDGET: usize = 16 * MIB;
const DEFAULT_BUDGET: usize = 128 * MIB;
const MAX_BUDGET: usize = 2 * 1024 * MIB;
const MAX_GPU_BUDGET: usize = 512 * MIB;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryPressure {
    #[default]
    Normal,
    Elevated,
    Critical,
}

/// A point-in-time observation supplied by an OS-specific adapter.
///
/// Process remaining is *remaining* allocatable headroom, not current usage.
/// Unknown values must be `None`, never made-up capacity. For integrated
/// graphics, GPU allocations also affect system memory; the caller must
/// account for that when budgeting staging and residency.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemorySample {
    pub physical_total_bytes: Option<usize>,
    pub physical_available_bytes: Option<usize>,
    pub process_remaining_bytes: Option<usize>,
    pub gpu_available_bytes: Option<usize>,
    pub logical_cpus: usize,
    pub pressure: MemoryPressure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceClass {
    Geometry,
    Undo,
    Preview,
    JobScratch,
    Transfer,
    Gpu,
}
impl ResourceClass {
    const fn ram_slot(self) -> Option<usize> {
        match self {
            Self::Geometry => Some(0),
            Self::Undo => Some(1),
            Self::Preview => Some(2),
            Self::JobScratch => Some(3),
            Self::Transfer => Some(4),
            Self::Gpu => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    pub ram_bytes: usize,
    pub gpu_bytes: usize,
    /// Maximum simultaneous *background* work requests. Zero means only
    /// foreground work may proceed, and new background work should defer.
    pub background_jobs: usize,
}
impl ResourceLimits {
    /// Conservative, bounded admission derived from an OS adapter's snapshot.
    /// This is an application quota, not total installed RAM or an OOM promise.
    pub fn from_sample(sample: MemorySample) -> Result<Self> {
        let mut cap = MAX_BUDGET;
        let mut observed = false;
        for (value, divisor) in [(sample.physical_total_bytes, 8), (sample.physical_available_bytes, 3), (sample.process_remaining_bytes, 3)] {
            if let Some(value) = value {
                observed = true;
                cap = cap.min(value / divisor);
            }
        }
        if !observed {
            cap = DEFAULT_BUDGET;
        }
        cap = match sample.pressure {
            MemoryPressure::Normal => cap,
            MemoryPressure::Elevated => cap / 2,
            MemoryPressure::Critical => cap / 4,
        };
        if cap < MIN_BUDGET {
            return Err(KernelError::Budget);
        }
        // Keep whole MiB quotas so per-class slices are deterministic.
        cap = (cap / MIB) * MIB;

        let cpu_count = sample.logical_cpus.max(1);
        let background_jobs = match sample.pressure {
            MemoryPressure::Normal => cpu_count.saturating_sub(1).min(8),
            MemoryPressure::Elevated => cpu_count.saturating_sub(1).min(2),
            MemoryPressure::Critical => 0,
        };
        // Unknown GPU budget intentionally disables tracked GPU allocations.
        // Software rendering remains a valid fallback.
        let gpu_bytes = sample.gpu_available_bytes.map_or(0, |b| (b / 4).min(MAX_GPU_BUDGET));
        Ok(Self { ram_bytes: cap, gpu_bytes, background_jobs })
    }

    /// The five CPU-side partitions sum to the total RAM quota:
    /// geometry 6/16, undo 4/16, preview 2/16, scratch 3/16,
    /// and transfers 1/16. GPU capacity is accounted separately.
    pub fn quota(self, class: ResourceClass) -> usize {
        let part = self.ram_bytes / 16;
        match class {
            ResourceClass::Geometry => part * 6,
            ResourceClass::Undo => part * 4,
            ResourceClass::Preview => part * 2,
            ResourceClass::JobScratch => part * 3,
            ResourceClass::Transfer => part,
            ResourceClass::Gpu => self.gpu_bytes,
        }
    }
}

#[derive(Debug)]
struct State {
    limits: ResourceLimits,
    ram_usage: [usize; 5],
    gpu_usage: usize,
}
impl State {
    fn used_ram(&self) -> Result<usize> {
        self.ram_usage.iter().try_fold(0usize, |total, n| total.checked_add(*n).ok_or(KernelError::Budget))
    }
    fn used(&self, class: ResourceClass) -> usize {
        match class.ram_slot() {
            Some(i) => self.ram_usage[i],
            None => self.gpu_usage,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourceLedger {
    inner: Arc<Mutex<State>>,
}
impl ResourceLedger {
    pub fn new(limits: ResourceLimits) -> Self {
        Self { inner: Arc::new(Mutex::new(State { limits, ram_usage: [0; 5], gpu_usage: 0 })) }
    }
    fn lock(&self) -> MutexGuard<'_, State> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }
    pub fn limits(&self) -> ResourceLimits {
        self.lock().limits
    }
    pub fn used(&self, class: ResourceClass) -> usize {
        self.lock().used(class)
    }
    /// Replanning never silently deallocates existing data. If the OS lowers
    /// our budget, existing leases remain alive, but new admission stops
    /// until callers release enough cache/undo/workspace resources.
    pub fn update_limits(&self, limits: ResourceLimits) {
        self.lock().limits = limits;
    }
    /// Reserves a declared capacity, not an actual allocation. Zero-byte
    /// requests are permitted and still return an owned lease.
    pub fn reserve(&self, class: ResourceClass, bytes: usize) -> Result<ResourceLease> {
        let mut state = self.lock();
        let new_class_usage = state.used(class).checked_add(bytes).ok_or(KernelError::Budget)?;
        if new_class_usage > state.limits.quota(class) {
            return Err(KernelError::Budget);
        }
        match class.ram_slot() {
            Some(i) => {
                if state.used_ram()?.checked_add(bytes).ok_or(KernelError::Budget)? > state.limits.ram_bytes {
                    return Err(KernelError::Budget);
                }
                state.ram_usage[i] = new_class_usage;
            }
            None => state.gpu_usage = new_class_usage,
        }
        Ok(ResourceLease { inner: self.inner.clone(), class, bytes })
    }
}

/// Unique RAII reservation. Sharing actual allocations must share this lease's
/// lifetime rather than making a new independent reservation.
#[derive(Debug)]
pub struct ResourceLease {
    inner: Arc<Mutex<State>>,
    class: ResourceClass,
    bytes: usize,
}
impl ResourceLease {
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
impl Drop for ResourceLease {
    fn drop(&mut self) {
        let mut state = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        match self.class.ram_slot() {
            Some(i) => state.ram_usage[i] -= self.bytes,
            None => state.gpu_usage -= self.bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(total_mib: usize, available_mib: usize, cpus: usize) -> MemorySample {
        MemorySample {
            physical_total_bytes: Some(total_mib * MIB),
            physical_available_bytes: Some(available_mib * MIB),
            logical_cpus: cpus,
            ..MemorySample::default()
        }
    }

    #[test]
    fn fallback_is_bounded_and_works_without_platform_probes() {
        let plan = ResourceLimits::from_sample(MemorySample::default()).unwrap();
        assert_eq!(plan.ram_bytes, 128 * MIB);
        assert_eq!(plan.gpu_bytes, 0);
        assert_eq!(plan.background_jobs, 0);
        assert_eq!(
            [ResourceClass::Geometry, ResourceClass::Undo, ResourceClass::Preview, ResourceClass::JobScratch, ResourceClass::Transfer]
                .iter()
                .map(|c| plan.quota(*c))
                .sum::<usize>(),
            plan.ram_bytes
        );
    }

    #[test]
    fn minimum_of_system_and_process_headroom_and_pressure() {
        let mut s = sample(16 * 1024, 6 * 1024, 12);
        s.process_remaining_bytes = Some(600 * MIB);
        let normal = ResourceLimits::from_sample(s).unwrap();
        assert_eq!(normal.ram_bytes, 200 * MIB);
        assert_eq!(normal.background_jobs, 8);
        s.pressure = MemoryPressure::Elevated;
        let elevated = ResourceLimits::from_sample(s).unwrap();
        assert_eq!(elevated.ram_bytes, 100 * MIB);
        assert_eq!(elevated.background_jobs, 2);
        s.pressure = MemoryPressure::Critical;
        let critical = ResourceLimits::from_sample(s).unwrap();
        assert_eq!(critical.ram_bytes, 50 * MIB);
        assert_eq!(critical.background_jobs, 0);
    }

    #[test]
    fn low_available_memory_blocks_new_work() {
        assert_eq!(ResourceLimits::from_sample(sample(8 * 1024, 12, 8)), Err(KernelError::Budget));
    }

    #[test]
    fn reservation_released_and_categories_remain_independent() {
        let limits = ResourceLimits::from_sample(MemorySample::default()).unwrap();
        let ledger = ResourceLedger::new(limits);
        let g = limits.quota(ResourceClass::Geometry);
        let lease = ledger.reserve(ResourceClass::Geometry, g).unwrap();
        assert!(ledger.reserve(ResourceClass::Geometry, 1).is_err());
        let undo = ledger.reserve(ResourceClass::Undo, 100).unwrap();
        assert_eq!(ledger.used(ResourceClass::Undo), 100);
        drop(lease);
        assert_eq!(ledger.used(ResourceClass::Geometry), 0);
        assert!(ledger.reserve(ResourceClass::Geometry, 100).is_ok());
        drop(undo);
        assert_eq!(ledger.used(ResourceClass::Undo), 0);
    }

    #[test]
    fn pressure_update_preserves_leases_and_stops_new_admission() {
        let normal = ResourceLimits::from_sample(sample(8 * 1024, 4 * 1024, 8)).unwrap();
        let ledger = ResourceLedger::new(normal);
        let old = ledger.reserve(ResourceClass::Preview, 40 * MIB).unwrap();
        let mut s = sample(8 * 1024, 256, 8);
        s.pressure = MemoryPressure::Elevated;
        let lower = ResourceLimits::from_sample(s).unwrap();
        ledger.update_limits(lower);
        assert_eq!(ledger.used(ResourceClass::Preview), 40 * MIB);
        assert!(ledger.reserve(ResourceClass::Preview, 1).is_err());
        drop(old);
        assert!(ledger.reserve(ResourceClass::Preview, 1).is_ok());
    }

    #[test]
    fn gpu_budget_is_explicit_and_independent() {
        let mut s = sample(8 * 1024, 4 * 1024, 8);
        s.gpu_available_bytes = Some(800 * MIB);
        let limits = ResourceLimits::from_sample(s).unwrap();
        assert_eq!(limits.gpu_bytes, 200 * MIB);
        let ledger = ResourceLedger::new(limits);
        let vram = ledger.reserve(ResourceClass::Gpu, 200 * MIB).unwrap();
        assert!(ledger.reserve(ResourceClass::Gpu, 1).is_err());
        assert!(ledger.reserve(ResourceClass::Geometry, 2 * MIB).is_ok());
        drop(vram);
        assert_eq!(ledger.used(ResourceClass::Gpu), 0);
    }

    #[test]
    fn simultaneous_reservation_is_atomic() {
        let plan = ResourceLimits::from_sample(MemorySample::default()).unwrap();
        let ledger = ResourceLedger::new(plan);
        let amount = plan.quota(ResourceClass::Transfer);
        let start = Arc::new(std::sync::Barrier::new(12));
        let release = Arc::new(std::sync::Barrier::new(12));
        let mut threads = Vec::new();
        for _ in 0..12 {
            let ledger = ledger.clone();
            let start = start.clone();
            let release = release.clone();
            threads.push(std::thread::spawn(move || {
                start.wait();
                let lease = ledger.reserve(ResourceClass::Transfer, amount).ok();
                // Every thread has tried before the winning lease is dropped.
                release.wait();
                lease.is_some()
            }));
        }
        let successful = threads.into_iter().map(|t| t.join().unwrap()).filter(|won| *won).count();
        assert_eq!(successful, 1);
        assert_eq!(ledger.used(ResourceClass::Transfer), 0);
    }
}
