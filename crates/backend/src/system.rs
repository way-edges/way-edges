use std::{
    cell::RefCell,
    collections::HashMap,
    fs::File,
    io::Read,
    rc::Rc,
    str::Lines,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use get_sys_info::{CPULoad, DelayedMeasurement, Filesystem, Platform};
use gio::glib::translate::Ptr;

const MEMORY_FILE: &str = "/proc/meminfo";
fn with_mem_info<T>(f: impl FnOnce(String) -> T) -> T {
    let s = File::open(MEMORY_FILE)
        .and_then(|mut f| {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            Ok(s)
        })
        .unwrap();
    f(s)
}

pub type RamInfo = [u64; 2];
pub static RAM_INFO: AtomicPtr<RamInfo> = AtomicPtr::new(std::ptr::null_mut());
fn update_ram_info(info: RamInfo) {
    RAM_INFO.store(
        Box::into_raw(Box::new(info)) as *mut _,
        std::sync::atomic::Ordering::Release,
    );
}
pub fn get_ram_info() -> Option<RamInfo> {
    unsafe {
        RAM_INFO
            .load(std::sync::atomic::Ordering::SeqCst)
            .as_ref()
            .cloned()
    }
}

pub type SwapInfo = RamInfo;
pub static SWAP_INFO: AtomicPtr<SwapInfo> = AtomicPtr::new(std::ptr::null_mut());
fn update_swap_info(info: SwapInfo) {
    SWAP_INFO.store(
        Box::into_raw(Box::new(info)) as *mut _,
        std::sync::atomic::Ordering::Release,
    );
}
pub fn get_swap_info() -> Option<SwapInfo> {
    unsafe {
        SWAP_INFO
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .cloned()
    }
}
static MEM_INITED: AtomicBool = AtomicBool::new(false);

fn ram_info(lines: &mut Lines) -> RamInfo {
    let mut free = None;
    let mut total = None;
    for ele in lines {
        if ele.starts_with("MemAvailable:") {
            free = Some(ele)
        } else if ele.starts_with("MemTotal:") {
            total = Some(ele)
        }
    }
    if let Some(free) = free {
        if let Some(total) = total {
            let free = free
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let total = total
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<u64>()
                .unwrap();

            [total - free, total]
        } else {
            panic!("MemTotal not found");
        }
    } else {
        panic!("MemAvailable not found");
    }
}

fn swap_info(lines: &mut Lines) -> SwapInfo {
    let mut free = None;
    let mut total = None;
    for ele in lines {
        if ele.starts_with("SwapFree:") {
            free = Some(ele);
            if total.is_some() {
                break;
            }
        } else if ele.starts_with("SwapTotal:") {
            total = Some(ele);
            if free.is_some() {
                break;
            }
        }
    }
    if let Some(free) = free {
        if let Some(total) = total {
            let free = free
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let total = total
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<u64>()
                .unwrap();

            [(total - free), total]
        } else {
            panic!("SwapTotal not found");
        }
    } else {
        panic!("SwapFree not found");
    }
}

pub fn init_mem_info() {
    if !MEM_INITED.load(std::sync::atomic::Ordering::Acquire) {
        fn update() {
            let (ram_info, swap_info) = with_mem_info(|s| {
                let mut lines = s.lines();
                let ram_info = ram_info(&mut lines.clone());
                let swap_info = swap_info(&mut lines);
                (ram_info, swap_info)
            });
            update_ram_info(ram_info);
            update_swap_info(swap_info);
        }
        update();
        gtk::glib::timeout_add_seconds(1, || {
            update();
            gio::glib::ControlFlow::Continue
        });
    }
}

pub type CpuInfo = (f64, f64);
struct SysInfo {
    cpu_info: Option<CpuInfo>,
    battery_info: Option<f64>,
    disk_map: HashMap<String, (u64, u64)>,
}

static SYS_INFO: AtomicPtr<SysInfo> = AtomicPtr::new(std::ptr::null_mut());
fn init_system() {
    if unsafe { SYS_INFO.as_ptr().as_ref().unwrap().is_null() } {
        SYS_INFO.store(
            Box::into_raw(Box::new(SysInfo {
                cpu_info: None,
                battery_info: None,
                disk_map: HashMap::new(),
            })) as *mut _,
            std::sync::atomic::Ordering::Release,
        );
    }
}
fn get_sys_info() -> Option<&'static mut SysInfo> {
    unsafe { SYS_INFO.load(std::sync::atomic::Ordering::Acquire).as_mut() }
}
fn update_cpu_info(info: CpuInfo) {
    if let Some(s) = get_sys_info() {
        s.cpu_info = Some(info);
    }
}
pub fn get_cpu_info() -> Option<CpuInfo> {
    unsafe {
        SYS_INFO
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .map(|s| s.cpu_info)?
    }
}
fn update_battery_info(info: f64) {
    if let Some(s) = get_sys_info() {
        s.battery_info = Some(info);
    }
}
pub fn get_battery_info() -> Option<f64> {
    unsafe {
        SYS_INFO
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .map(|s| s.battery_info)?
    }
}
fn update_disk_info(mut f: impl FnMut(&str) -> (u64, u64)) {
    if let Some(s) = get_sys_info() {
        s.disk_map.iter_mut().for_each(|(k, v)| *v = f(k.as_str()));
    }
}
fn filesys_2_percent(f: Filesystem) -> (u64, u64) {
    let a = f.avail.0 / 1000;
    let t = f.total.0 / 1000;
    (t - a, t)
}
pub fn register_disk_partition(s: &str) {
    if let Some(info) = get_sys_info() {
        let sys = get_sys_info::System::new();
        if let Ok(f) = sys.mount_at(s) {
            info.disk_map.insert(s.to_string(), filesys_2_percent(f));
        }
    };
}
pub fn get_disk_info(s: &str) -> Option<(u64, u64)> {
    get_sys_info().map(|sys| sys.disk_map.get(s).cloned())?
}

use get_sys_info::platform::linux::PlatformImpl;
fn cpu_info(sys: &PlatformImpl, holder: &Rc<RefCell<DelayedMeasurement<CPULoad>>>) {
    let mut re = holder.borrow_mut();
    let cpu_load = re.done().unwrap();
    let temp = sys.cpu_temp().unwrap_or_default();
    update_cpu_info(((1. - cpu_load.idle).into(), temp.into()));
    *re = sys.cpu_load_aggregate().unwrap();
}
fn battery_info(sys: &PlatformImpl) {
    if let Ok(r) = sys.battery_life() {
        update_battery_info(r.remaining_capacity.into());
    }
}
fn disk_info(sys: &PlatformImpl) {
    update_disk_info(|s| sys.mount_at(s).map(filesys_2_percent).unwrap());
}

static SYSTEM_INITED: AtomicBool = AtomicBool::new(false);
pub fn init_system_info() {
    if !SYSTEM_INITED.load(std::sync::atomic::Ordering::Acquire) {
        init_system();
        let sys = get_sys_info::System::new();
        let progress_holder = Rc::new(RefCell::new(sys.cpu_load_aggregate().unwrap()));
        battery_info(&sys);
        disk_info(&sys);
        gtk::glib::timeout_add_seconds_local(1, move || {
            cpu_info(&sys, &progress_holder);
            battery_info(&sys);
            disk_info(&sys);
            gio::glib::ControlFlow::Continue
        });
    }
}
