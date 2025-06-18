use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind};

#[derive(Debug, Clone)]
pub enum MouseEvent {
    Press((f64, f64), u32),
    Release((f64, f64), u32),
    Enter((f64, f64)),
    Leave,
    Motion((f64, f64)),
    Scroll(f64, f64), // horizontal, vertical
}

#[derive(Debug)]
pub struct MouseStateData {
    pub hovering: bool,
    pub pressing: Option<u32>,
}
impl MouseStateData {
    pub fn new() -> Self {
        Self {
            hovering: false,
            pressing: None,
        }
    }
}
impl Default for MouseStateData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct MouseState {
    pub data: MouseStateData,
    mouse_debug: bool,
}
impl MouseState {
    pub fn is_hovering(&self) -> bool {
        self.data.hovering
    }
}

impl MouseState {
    pub fn new() -> Self {
        Self {
            data: MouseStateData::new(),
            mouse_debug: false,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_wl_pointer(&mut self, event: &PointerEvent) -> Option<MouseEvent> {
        use PointerEventKind::*;
        match event.kind {
            Enter { .. } => self.hover_enter(event.position),
            Leave { .. } => self.hover_leave(),
            Motion { .. } => self.hover_motion(event.position),
            Press { button, .. } => self.press(button, event.position),
            Release { button, .. } => self.unpress(button, event.position),
            Axis {
                horizontal,
                vertical,
                ..
            } => Some(MouseEvent::Scroll(horizontal.absolute, vertical.absolute)),
        }
    }

    // triggers
    fn press(&mut self, p: u32, pos: (f64, f64)) -> Option<MouseEvent> {
        if self.mouse_debug {
            let msg = format!("key pressed: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            util::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if self.data.pressing.is_none() {
            self.data.pressing = Some(p);
            Some(MouseEvent::Press(pos, p))
        } else {
            None
        }
    }
    fn unpress(&mut self, p: u32, pos: (f64, f64)) -> Option<MouseEvent> {
        if self.mouse_debug {
            let msg = format!("key released: {}", p);
            log::debug!("Mouse Debug info: {msg}");
            util::notify_send("Way-edges mouse button debug message", &msg, false);
        };

        if self.data.pressing.eq(&Some(p)) {
            self.data.pressing = None;
            Some(MouseEvent::Release(pos, p))
        } else {
            None
        }
    }
    fn hover_enter(&mut self, pos: (f64, f64)) -> Option<MouseEvent> {
        self.data.hovering = true;
        Some(MouseEvent::Enter(pos))
    }
    fn hover_motion(&mut self, pos: (f64, f64)) -> Option<MouseEvent> {
        Some(MouseEvent::Motion(pos))
    }
    fn hover_leave(&mut self) -> Option<MouseEvent> {
        self.data.hovering = false;
        Some(MouseEvent::Leave)
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new()
    }
}
