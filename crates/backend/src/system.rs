use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use starship_battery::Battery;
use sysinfo::{Disks, MemoryRefreshKind, System};

static SYSTEM: LazyLock<Arc<Mutex<System>>> = LazyLock::new(|| Arc::new(Mutex::new(System::new())));

fn get_system() -> MutexGuard<'static, System> {
    Mutex::lock(&SYSTEM).unwrap()
}

pub struct MemoryInfo {
    pub free: u64,
    pub total: u64,
}

pub fn get_ram_info() -> MemoryInfo {
    let mut sys = get_system();
    sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    MemoryInfo {
        free: sys.free_memory(),
        total: sys.total_memory(),
    }
}

pub fn get_swap_info() -> MemoryInfo {
    let mut sys = get_system();
    sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_swap());
    MemoryInfo {
        free: sys.free_swap(),
        total: sys.total_swap(),
    }
}

pub fn get_cpu_info(core: Option<usize>) -> f64 {
    let mut sys = get_system();
    sys.refresh_cpu_usage();
    if let Some(core_id) = core {
        sys.cpus().get(core_id).unwrap().cpu_usage() as f64
    } else {
        sys.global_cpu_usage() as f64
    }
}

static BATTERY: LazyLock<Arc<Mutex<Battery>>> = LazyLock::new(|| {
    let manager = starship_battery::Manager::new().unwrap();
    let battery = manager.batteries().unwrap().next().unwrap().unwrap();
    Arc::new(Mutex::new(battery))
});

fn get_battery() -> MutexGuard<'static, Battery> {
    Mutex::lock(&BATTERY).unwrap()
}

pub fn get_battery_info() -> f64 {
    let mut battery = get_battery();
    battery.refresh().unwrap();
    use starship_battery::units::ratio::ratio;
    battery.state_of_charge().get::<ratio>() as f64
}

static DISK: LazyLock<Arc<Mutex<Disks>>> = LazyLock::new(|| Arc::new(Mutex::new(Disks::new())));

fn get_disk() -> MutexGuard<'static, Disks> {
    Mutex::lock(&DISK).unwrap()
}
pub struct DiskInfo {
    pub free: u64,
    pub total: u64,
}
pub fn get_disk_info(partition: &str) -> DiskInfo {
    let mut disk = get_disk();
    disk.refresh_specifics(true, sysinfo::DiskRefreshKind::nothing().with_storage());

    let partition = disk
        .iter()
        .find(|d| d.mount_point().to_str().unwrap() == partition)
        .unwrap();
    DiskInfo {
        free: partition.available_space(),
        total: partition.total_space(),
    }
}
