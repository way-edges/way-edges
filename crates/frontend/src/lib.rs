mod animation;
mod buffer;
mod frame;
mod mouse_state;
pub mod widgets;
// pub mod window;

mod wayland;

pub use wayland::mainloop::run_app;
