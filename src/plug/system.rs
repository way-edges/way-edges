use std::{
    cell::RefCell,
    fs::File,
    io::Read,
    rc::Rc,
    str::Lines,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use get_sys_info::Platform;

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

pub type RamInfo = (u64, u64);
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
            .load(std::sync::atomic::Ordering::Acquire)
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
    let mut ava = None;
    let mut total = None;
    for ele in lines {
        if ele.starts_with("MemAvailable:") {
            ava = Some(ele)
        } else if ele.starts_with("MemTotal:") {
            total = Some(ele)
        }
    }
    if let Some(ava) = ava {
        if let Some(total) = total {
            let ava = ava
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

            (ava, total)
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

            ((total - free), total)
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
static CPU_INFO: AtomicPtr<CpuInfo> = AtomicPtr::new(std::ptr::null_mut());
fn update_cpu_info(info: CpuInfo) {
    CPU_INFO.store(
        std::ptr::addr_of!(info) as *mut _,
        std::sync::atomic::Ordering::Release,
    );
}
pub fn get_system() -> Option<CpuInfo> {
    unsafe {
        CPU_INFO
            .load(std::sync::atomic::Ordering::Acquire)
            .as_ref()
            .cloned()
    }
}

static CPU_INITED: AtomicBool = AtomicBool::new(false);
pub fn init_system() {
    if !CPU_INITED.load(std::sync::atomic::Ordering::Acquire) {
        let sys = get_sys_info::System::new();
        let holder = Rc::new(RefCell::new(sys.cpu_load_aggregate().unwrap()));
        gtk::glib::timeout_add_seconds_local(1, move || {
            let re = holder.borrow_mut();
            let a = re.done().unwrap();
            println!("{a:#?}");

            // update_cpu_info();
            *holder.borrow_mut() = sys.cpu_load_aggregate().unwrap();
            gio::glib::ControlFlow::Continue
        });
    }
}

pub fn get_cpu_usage() {
    let s = File::open("/proc/stat")
        .and_then(|mut f| {
            let mut s = String::new();
            f.read_to_string(&mut s)?;
            Ok(s)
        })
        .unwrap();
    let cpu = s.lines().find(|line| line.starts_with("cpu ")).unwrap();
}
