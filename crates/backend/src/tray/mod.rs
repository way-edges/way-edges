mod context;
mod event;
pub mod icon;

pub use context::{init_tray_client, register_tray, unregister_tray};
pub use event::{tray_request_event, Event};
