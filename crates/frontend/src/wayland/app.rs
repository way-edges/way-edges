use std::{
    collections::HashMap,
    mem,
    rc::Rc,
    sync::{atomic::AtomicPtr, Arc, Mutex, Weak},
    time::Duration,
};

use backend::ipc::IPCCommand;
use calloop::{
    channel::Sender,
    ping::{make_ping, Ping},
    Idle, LoopHandle, LoopSignal,
};
use config::{common::MonitorSpecifier, shared::Curve};
use smithay_client_toolkit::{
    compositor::{CompositorState, SurfaceData as SctkSurfaceData, SurfaceDataExt},
    output::OutputState,
    reexports::protocols::wp::{
        fractional_scale::v1::client::{
            wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
            wp_fractional_scale_v1::WpFractionalScaleV1,
        },
        viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    },
    registry::{GlobalProxy, RegistryState},
    seat::{pointer::PointerEvent, SeatState},
    shell::{
        wlr_layer::{LayerShell, LayerSurface},
        WaylandSurface,
    },
    shm::{slot::SlotPool, Shm},
};
use wayland_client::{
    protocol::{wl_output::WlOutput, wl_pointer, wl_surface::WlSurface},
    Proxy, QueueHandle,
};

use crate::{
    animation::{AnimationList, ToggleAnimation, ToggleAnimationRc},
    buffer::Buffer,
    mouse_state::{MouseEvent, MouseState},
    widgets::{init_widget, WidgetContext},
};

use super::{draw::DrawCore, window_pop_state::WindowPopState};

pub struct App {
    pub exit: bool,
    pub show_mouse_key: bool,
    pub widget_map: WidgetMap,

    pub queue_handle: QueueHandle<App>,
    pub event_loop_handle: LoopHandle<'static, App>,
    pub signal: LoopSignal,

    pub compositor_state: CompositorState,
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
    pub fractional_manager: GlobalProxy<WpFractionalScaleManagerV1>,
    pub viewporter_manager: GlobalProxy<WpViewporter>,
    pub pointer: Option<wl_pointer::WlPointer>,

    pub shell: LayerShell,
    pub shm: Shm,
    pub pool: SlotPool,

    // if the the outputs get updated before we first initialize widgets, do not call reload
    pub(crate) first_time_initialized: bool,
    pub(crate) reload_guard: Option<Idle<'static>>,
}
impl App {
    pub fn handle_ipc(&mut self, cmd: IPCCommand) {
        match cmd {
            IPCCommand::TogglePin(wn) => self.toggle_pin(&wn),
            IPCCommand::Exit => self.exit = true,
            IPCCommand::Reload => self.reload(),
        };
    }

    fn toggle_pin(&mut self, name: &str) {
        for w in self.widget_map.get_widgets(name) {
            w.lock().unwrap().toggle_pin(self)
        }
    }

    fn reload_widgets(&mut self) {
        // clear contents of old widgets
        let ws = mem::take(&mut self.widget_map.0)
            .into_values()
            .flatten()
            .collect::<Vec<_>>();
        ws.into_iter().for_each(|arc| {
            // we make sure that no other references exist
            // tipically this should be Some() since this function is called in idle
            // and the backend or any other threads shall not hold references to widgets
            let mtx = Arc::into_inner(arc).unwrap();

            // and tipically this should be Ok() since no other references should exist
            match mtx.into_inner() {
                Ok(mut w) => w.clear_contents(self),
                Err(e) => {
                    log::error!(
                        "Failed to clear widget contents during reload, mutex of this widget is poisoned: {e}"
                    );
                }
            }
        });

        // create new
        self.widget_map = config::get_config_root()
            .and_then(|c| WidgetMap::new(c.widgets.clone(), self))
            .unwrap_or_else(|e| {
                log::error!("Failed to load widgets: {e}");
                WidgetMap(HashMap::new())
            });
    }

    pub fn reload(&mut self) {
        log::info!("Reloading widgets...");
        self.first_time_initialized = true;

        let idle = self.event_loop_handle.insert_idle(App::reload_widgets);
        if let Some(old) = self.reload_guard.replace(idle) {
            old.cancel()
        }
    }
}

// the states from `App` that are needed when building widgets
pub struct WidgetBuildingStates<'a> {
    pub event_loop_handle: &'a LoopHandle<'static, App>,
    pub output_state: &'a OutputState,
}

#[derive(Debug, Default)]
pub struct WidgetMap(HashMap<String, Vec<Arc<Mutex<Widget>>>>);
impl WidgetMap {
    fn new(widgets_config: Vec<config::Widget>, app: &App) -> Result<Self, String> {
        let mut map: HashMap<String, Vec<Arc<Mutex<Widget>>>> = HashMap::new();

        for conf in widgets_config.iter().cloned() {
            let name = conf.common.namespace.clone();
            let confs: Vec<(config::Widget, WlOutput)> = match conf.common.monitor.clone() {
                MonitorSpecifier::ID(index) => app
                    .output_state
                    .outputs()
                    .nth(index)
                    .map(|output| vec![(conf, output)])
                    .unwrap_or(vec![]),
                MonitorSpecifier::Names(items) => app
                    .output_state
                    .outputs()
                    .filter_map(|out| {
                        app.output_state
                            .info(&out)
                            .and_then(|info| info.name)
                            .filter(|output_name| items.contains(output_name))
                            .is_some()
                            .then(|| (conf.clone(), out))
                    })
                    .collect(),
                MonitorSpecifier::All => app
                    .output_state
                    .outputs()
                    .map(|out| (conf.clone(), out))
                    .collect(),
                _ => unreachable!(),
            };

            for (conf, output) in confs.into_iter() {
                let ctx = Widget::init_widget(conf, output, app)?;
                // TODO: CLONE
                map.entry(name.clone()).or_default().push(ctx.clone());
            }
        }

        Ok(Self(map))
    }

    fn get_widgets(&self, name: &str) -> Vec<Arc<Mutex<Widget>>> {
        self.0.get(name).cloned().unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct Widget {
    pub monitor: MonitorSpecifier,
    pub configured: bool,

    pub output: WlOutput,
    pub layer: LayerSurface,
    pub scale: Scale,

    pub mouse_state: MouseState,
    pub window_pop_state: WindowPopState,
    pub start_pos: (i32, i32),

    pub w: Box<dyn WidgetContext>,
    pub buffer: Buffer,
    pub content_width: i32,
    pub content_height: i32,
    pub draw_core: DrawCore,

    pub pop_animation: ToggleAnimationRc,
    pub animation_list: AnimationList,

    pop_animation_finished: bool,
    widget_animation_finished: bool,

    widget_has_update: bool,
    next_frame: bool,
    frame_available: bool,

    offset: i32,
    margins: [i32; 4],

    // for damage
    output_size: (i32, i32),
}
impl Widget {
    fn call_frame(&mut self, qh: &QueueHandle<App>) {
        self.frame_available = false;
        self.layer
            .wl_surface()
            .frame(qh, self.layer.wl_surface().clone());
    }
    pub fn on_frame_callback(&mut self, app: &mut App) {
        if self.has_animation_update() || self.next_frame {
            self.draw(app);
        } else {
            self.frame_available = true;
        }
    }
    fn on_widget_update(&mut self, app: &mut App) {
        self.widget_has_update = true;
        self.try_redraw(app);
    }
    fn try_redraw(&mut self, app: &mut App) {
        if !self.configured {
            return;
        }

        if self.frame_available {
            self.draw(app)
        } else {
            self.next_frame = true;
        }
    }
    fn has_animation_update(&mut self) -> bool {
        let widget_has_animation_update = self.animation_list.has_in_progress();
        let pop_animation_update = self.pop_animation.borrow().is_in_progress();

        widget_has_animation_update
            || !self.widget_animation_finished
            || pop_animation_update
            || !self.pop_animation_finished
    }
    fn prepare_content(&mut self) {
        self.animation_list.refresh();
        self.pop_animation.borrow_mut().refresh();

        if self.pop_animation.borrow().is_in_progress() {
            if self.pop_animation_finished {
                self.pop_animation_finished = false
            }
        } else if !self.pop_animation_finished {
            self.pop_animation_finished = true;
        }

        let widget_has_animation_update = if self.animation_list.has_in_progress() {
            if self.widget_animation_finished {
                self.widget_animation_finished = false
            }
            true
        } else if !self.widget_animation_finished {
            self.widget_animation_finished = true;
            true
        } else {
            false
        };

        // update content
        if self.widget_has_update || widget_has_animation_update {
            self.widget_has_update = false;
            let img = self.w.redraw();
            self.content_width = img.width();
            self.content_height = img.height();
            self.buffer.update_buffer(img);
        }
    }
    pub fn draw(&mut self, app: &mut App) {
        if self.next_frame {
            self.next_frame = false
        }
        self.prepare_content();

        let progress = self.pop_animation.borrow_mut().progress();
        let coordinate = self.draw_core.calc_coordinate(
            (self.content_width, self.content_height),
            self.offset,
            progress,
        );
        self.start_pos = (coordinate[0], coordinate[1]);
        let width = coordinate[2];
        let height = coordinate[3];

        // create and draw content
        let (buffer, canvas) = app
            .pool
            .create_buffer(
                width,
                height,
                width * 4,
                wayland_client::protocol::wl_shm::Format::Argb8888,
            )
            .unwrap();
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");
        // clear old buffer*
        canvas.fill(0);

        // copy with transition
        let buffer = self.buffer.get_buffer();
        buffer
            .with_data(|data| {
                util::draw::copy_pixmap(
                    data,
                    buffer.width() as usize,
                    buffer.height() as usize,
                    canvas,
                    width as usize,
                    height as usize,
                    coordinate[0] as isize,
                    coordinate[1] as isize,
                );
            })
            .unwrap();

        // attach content
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.output_size.0, self.output_size.1);

        // set size
        let (w, h) = self.scale.calculate_size(width as u32, height as u32);
        self.layer.set_size(w, h);

        self.call_frame(&app.queue_handle);

        self.layer.commit();
    }

    fn toggle_pin(&mut self, app: &mut App) {
        self.window_pop_state
            .toggle_pin(self.mouse_state.is_hovering());
        self.try_redraw(app);
    }
    pub fn update_normal(&mut self, normal: u32, app: &mut App) {
        // IGNORING NORMAL SCALE IF FRACTIONAL SCALE IS AVAILABLE
        if self.scale.is_fractional() {
            return;
        }

        if self.scale.update_normal(normal) {
            self.try_redraw(app);
        }
        let margins = self.scale.calculate_margin(self.margins);
        self.layer
            .set_margin(margins[0], margins[1], margins[2], margins[3]);
    }
    pub fn update_fraction(&mut self, fraction: u32, app: &mut App) {
        if self.scale.update_fraction(fraction) {
            self.try_redraw(app);
        }
        let margins = self.scale.calculate_margin(self.margins);
        self.layer
            .set_margin(margins[0], margins[1], margins[2], margins[3]);
    }
    pub fn on_mouse_event(&mut self, app: &mut App, event: &PointerEvent) {
        let Some(mut event) = self.mouse_state.from_wl_pointer(event) else {
            return;
        };

        let data = &mut self.mouse_state.data;

        let mut trigger_redraw = false;
        let mut do_redraw = || {
            if !trigger_redraw {
                trigger_redraw = true;
            }
        };

        match &mut event {
            MouseEvent::Release(pos, _)
            | MouseEvent::Press(pos, _)
            | MouseEvent::Enter(pos)
            | MouseEvent::Motion(pos) => {
                self.scale.calculate_pos(pos);
                pos.0 -= self.start_pos.0 as f64;
                pos.1 -= self.start_pos.1 as f64;
            }
            _ => {}
        }

        match event {
            MouseEvent::Release(_, key) => {
                if self
                    .window_pop_state
                    .toggle_pin_with_key(key, data.hovering)
                {
                    do_redraw()
                }
            }
            MouseEvent::Enter(_) => {
                self.window_pop_state.enter();
                do_redraw()
            }
            MouseEvent::Leave => {
                self.window_pop_state.leave();
                do_redraw()
            }
            MouseEvent::Motion(_) => self.window_pop_state.invalidate_pop(),
            _ => {}
        }

        let widget_trigger_redraw = self.w.on_mouse_event(data, event);

        if widget_trigger_redraw {
            self.on_widget_update(app);
        } else if trigger_redraw {
            self.try_redraw(app);
        }
    }

    fn init_widget(
        conf: config::Widget,
        wl_output: WlOutput,
        app: &App,
    ) -> Result<Arc<Mutex<Self>>, String> {
        let config::Widget { common, widget } = conf;
        let mut builder = WidgetBuilder::new(common, wl_output, app)?;
        let w = init_widget(widget, &mut builder);
        let s = builder.build(w);

        Ok(Arc::new_cyclic(|weak| {
            SurfaceData::from_wl(s.layer.wl_surface()).store_widget(weak.clone());
            Mutex::new(s)
        }))
    }

    fn clear_contents(&mut self, app: &mut App) {
        let (buffer, canvas) = app
            .pool
            .create_buffer(1, 1, 4, wayland_client::protocol::wl_shm::Format::Argb8888)
            .unwrap();
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");
        canvas.fill(0);

        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.output_size.0, self.output_size.1);

        self.call_frame(&app.queue_handle);

        self.layer.commit();
    }
}

#[derive(Debug)]
pub struct Scale {
    normal: u32,
    fractional: Option<(u32, WpFractionalScaleV1, WpViewport)>,
}
impl Scale {
    fn new_fractional(fractional_client: WpFractionalScaleV1, viewprot: WpViewport) -> Self {
        Self {
            normal: 1,
            fractional: Some((0, fractional_client, viewprot)),
        }
    }
    fn new_normal() -> Self {
        Self {
            normal: 1,
            fractional: None,
        }
    }
    fn is_fractional(&self) -> bool {
        self.fractional.is_some()
    }
    fn update_normal(&mut self, normal: u32) -> bool {
        let changed = self.normal != normal;
        self.normal = normal;
        changed
    }
    fn update_fraction(&mut self, fraction: u32) -> bool {
        if let Some(fractional) = self.fractional.as_mut() {
            let changed = fractional.0 != fraction;
            fractional.0 = fraction;
            changed
        } else {
            false
        }
    }
    fn calculate_size(&self, width: u32, height: u32) -> (u32, u32) {
        if let Some(fractional) = self.fractional.as_ref() {
            let mut scale = fractional.0;
            if scale == 0 {
                scale = 120
            }
            let size = (
                ((width * 120 + 60) / scale).max(1),
                ((height * 120 + 60) / scale).max(1),
            );

            // viewport
            fractional.2.set_destination(size.0 as i32, size.1 as i32);

            size
        } else {
            (width / self.normal, height / self.normal)
        }
    }
    fn calculate_pos(&self, pos: &mut (f64, f64)) {
        if let Some(fractional) = self.fractional.as_ref() {
            let mut scale = fractional.0;
            if scale == 0 {
                scale = 120
            }
            let scale_f64 = scale as f64 / 120.;
            pos.0 *= scale_f64;
            pos.1 *= scale_f64;
        } else {
            pos.0 *= self.normal as f64;
            pos.1 *= self.normal as f64;
        }
    }
    fn calculate_margin(&self, margins: [i32; 4]) -> [i32; 4] {
        let c = |m: i32| {
            (if let Some(fractional) = self.fractional.as_ref() {
                let mut scale = fractional.0;
                if scale == 0 {
                    scale = 120
                }
                (m as u32 * 120 + 60) / scale
            } else {
                m as u32 / self.normal
            }) as i32
        };
        [c(margins[0]), c(margins[1]), c(margins[2]), c(margins[3])]
    }
}
impl Drop for Scale {
    fn drop(&mut self) {
        #[allow(clippy::option_map_unit_fn)]
        self.fractional.as_ref().map(|(_, f, v)| {
            f.destroy();
            v.destroy();
        });
    }
}

macro_rules! widget_from_layer {
    ($w:ident, $layer:expr) => {
        let Some($w) = SurfaceData::from_wl($layer.wl_surface()).get_widget() else {
            return;
        };
    };
    ($w:ident, $layer:expr, $ret:expr) => {
        let Some($w) = SurfaceData::from_wl($layer.wl_surface()).get_widget() else {
            return $ret;
        };
    };
}

struct PopEssential {
    pop_duration: Duration,
    layer: LayerSurface,
}
impl PopEssential {
    fn pop(&self, app: &mut App) {
        // pop up
        let guard_weak = {
            widget_from_layer!(w, self.layer);

            let mut wg = w.lock().unwrap();
            let state = &mut wg.window_pop_state;
            state.enter();

            let guard = Rc::new(());
            let guard_weak = Rc::downgrade(&guard);
            state.pop_state.replace(guard);

            wg.try_redraw(app);

            guard_weak
        };

        // hide
        let layer = self.layer.clone();
        app.event_loop_handle
            .insert_source(
                calloop::timer::Timer::from_duration(self.pop_duration),
                move |_, _, app| {
                    if guard_weak.upgrade().is_none() {
                        return calloop::timer::TimeoutAction::Drop;
                    }

                    widget_from_layer!(w, layer, calloop::timer::TimeoutAction::Drop);

                    let mut wg = w.lock().unwrap();
                    if !wg.mouse_state.data.hovering {
                        wg.window_pop_state.leave();
                        wg.try_redraw(app);
                    }

                    calloop::timer::TimeoutAction::Drop
                },
            )
            .unwrap();
    }
}

struct RedrawEssentail {
    layer: LayerSurface,
}
impl RedrawEssentail {
    fn redraw(&self, app: &mut App) {
        let Some(w) = SurfaceData::from_wl(self.layer.wl_surface()).get_widget() else {
            return;
        };
        w.lock().unwrap().on_widget_update(app);
    }
}

pub struct WidgetBuilder<'a> {
    pub common_config: config::CommonConfig,

    pub offset: i32,
    pub margins: [i32; 4],
    pub output_size: (i32, i32),

    pub monitor: MonitorSpecifier,
    pub output: WlOutput,
    pub app: WidgetBuildingStates<'a>,
    pub layer: LayerSurface,
    pub scale: Scale,

    pub animation_list: AnimationList,
    pub window_pop_state: WindowPopState,
}
impl WidgetBuilder<'_> {
    pub fn new_animation(&mut self, time_cost: u64, curve: Curve) -> ToggleAnimationRc {
        self.animation_list.new_transition(time_cost, curve)
    }
    pub fn extend_animation_list(&mut self, list: &AnimationList) {
        self.animation_list.extend_list(list);
    }
    fn make_pop_essential(&self, pop_duration: u64) -> PopEssential {
        let layer = self.layer.clone();
        let pop_duration = Duration::from_millis(pop_duration);
        PopEssential {
            pop_duration,
            layer,
        }
    }
    pub fn make_pop_channel<T: 'static>(
        &mut self,
        pop_duration: u64,
        mut func: impl FnMut(&mut App, T) + 'static,
    ) -> Sender<T> {
        let (sender, source) = calloop::channel::channel();

        let pop_essential = self.make_pop_essential(pop_duration);
        self.app
            .event_loop_handle
            .insert_source(source, move |event, _, app| {
                if let calloop::channel::Event::Msg(msg) = event {
                    func(app, msg);
                    pop_essential.pop(app);
                }
            })
            .unwrap();

        sender
    }
    pub fn make_pop_ping_with_func(
        &mut self,
        pop_duration: u64,
        mut func: impl FnMut(&mut App) + 'static,
    ) -> Ping {
        let (ping, source) = make_ping().unwrap();

        let pop_essential = self.make_pop_essential(pop_duration);
        self.app
            .event_loop_handle
            .insert_source(source, move |_, _, app| {
                func(app);
                pop_essential.pop(app);
            })
            .unwrap();

        ping
    }
    pub fn make_pop_ping(&mut self, pop_duration: u64) -> Ping {
        let (ping, source) = make_ping().unwrap();

        let pop_essential = self.make_pop_essential(pop_duration);
        self.app
            .event_loop_handle
            .insert_source(source, move |_, _, app| {
                pop_essential.pop(app);
            })
            .unwrap();

        ping
    }

    fn make_redraw_essentail(&self) -> RedrawEssentail {
        let layer = self.layer.clone();
        RedrawEssentail { layer }
    }
    pub fn make_redraw_channel<T: 'static>(
        &self,
        mut func: impl FnMut(&mut App, T) + 'static,
    ) -> Sender<T> {
        let (sender, source) = calloop::channel::channel();

        let redraw_essential = self.make_redraw_essentail();
        self.app
            .event_loop_handle
            .insert_source(source, move |event, _, app| {
                if let calloop::channel::Event::Msg(msg) = event {
                    func(app, msg);
                    redraw_essential.redraw(app);
                }
            })
            .unwrap();

        sender
    }
    pub fn make_redraw_ping_with_func(&self, mut func: impl FnMut(&mut App) + 'static) -> Ping {
        let (ping, source) = make_ping().unwrap();

        let redraw_essential = self.make_redraw_essentail();
        self.app
            .event_loop_handle
            .insert_source(source, move |_, _, app| {
                func(app);
                redraw_essential.redraw(app);
            })
            .unwrap();

        ping
    }
    pub fn make_redraw_ping(&self) -> Ping {
        let (ping, source) = make_ping().unwrap();

        let redraw_essential = self.make_redraw_essentail();
        self.app
            .event_loop_handle
            .insert_source(source, move |_, _, app| {
                redraw_essential.redraw(app);
            })
            .unwrap();
        ping
    }
}
impl<'a> WidgetBuilder<'a> {
    fn new(
        mut common: config::CommonConfig,
        output: WlOutput,
        app: &'a App,
    ) -> Result<WidgetBuilder<'a>, String> {
        let monitor = app
            .output_state
            .info(&output)
            .ok_or("Failed to get output info")?;
        let output_size = monitor.modes[0].dimensions;
        common.resolve_relative(output_size);

        let surface = app.compositor_state.create_surface_with_data(
            &app.queue_handle,
            SurfaceData {
                sctk: SctkSurfaceData::new(None, 1),
                widget: AtomicPtr::new(std::ptr::null_mut()),
            },
        );
        let fractional = app
            .fractional_manager
            .get()
            .inspect_err(|e| log::error!("Fatal on Fractional scale: {e}"))
            .ok()
            .map(|manager| {
                manager.get_fractional_scale(&surface, &app.queue_handle, surface.clone())
            })
            .and_then(|fractional| {
                app.viewporter_manager
                    .get()
                    .inspect_err(|e| {
                        // NOTE: DESTROY FRACTIONAL IF WE FAILED TO GET VIEWPORT
                        fractional.destroy();
                        log::error!("Fatal on Viewporter: {e}");
                    })
                    .ok()
                    .map(|manager| {
                        (
                            fractional,
                            manager.get_viewport(&surface, &app.queue_handle, ()),
                        )
                    })
            });
        let scale = match fractional {
            Some((f, v)) => Scale::new_fractional(f, v),
            None => Scale::new_normal(),
        };

        let layer = app.shell.create_layer_surface(
            &app.queue_handle,
            surface,
            common.layer,
            Some(format!("way-edges-widget{}", common.namespace)),
            Some(&output),
        );
        layer.set_anchor(common.edge | common.position);
        if common.ignore_exclusive {
            layer.set_exclusive_zone(-1);
        };

        let offset = common.offset.get_num().unwrap() as i32;

        let margins = [
            common.margins.top.get_num().unwrap() as i32,
            common.margins.right.get_num().unwrap() as i32,
            common.margins.bottom.get_num().unwrap() as i32,
            common.margins.left.get_num().unwrap() as i32,
        ];
        layer.set_margin(margins[0], margins[1], margins[2], margins[3]);
        layer.set_size(1, 1);
        layer.commit();

        let pop_animation = ToggleAnimation::new(
            Duration::from_millis(common.transition_duration),
            common.animation_curve,
        )
        .make_rc();
        let animation_list = AnimationList::new();
        let mut window_pop_state = WindowPopState::new(
            pop_animation,
            common.pinnable,
            common.pin_with_key,
            common.pin_key,
        );

        if common.pin_on_startup {
            window_pop_state.toggle_pin(false);
        }

        let widget_builder_states = WidgetBuildingStates {
            event_loop_handle: &app.event_loop_handle,
            output_state: &app.output_state,
        };

        Ok(Self {
            monitor: common.monitor.clone(),
            output,
            app: widget_builder_states,
            layer,
            animation_list,
            scale,
            offset,
            margins,
            output_size,
            window_pop_state,
            common_config: common,
        })
    }
    pub fn build(self, w: Box<dyn WidgetContext>) -> Widget {
        let Self {
            monitor,
            output,
            app: _,
            layer,
            scale,
            animation_list,
            offset,
            margins,
            output_size,
            window_pop_state,
            common_config,
        } = self;

        let start_pos = (0, 0);
        let mouse_state = MouseState::new();
        let buffer = Buffer::default();
        let draw_core = DrawCore::new(&common_config);

        Widget {
            monitor,
            configured: false,
            output,
            layer,
            scale,
            pop_animation: window_pop_state.pop_animation.clone(),
            animation_list,
            mouse_state,
            window_pop_state,
            start_pos,
            w,
            buffer,
            draw_core,
            pop_animation_finished: true,
            widget_animation_finished: true,
            content_width: 1,
            content_height: 1,
            widget_has_update: true,
            next_frame: false,
            frame_available: true,
            offset,
            margins,
            output_size,
        }
    }
}

// TODO: we are not really access this in multithreaded situation, so we don't need
// Arc&Mutex, but since WlSurface::data needs Send&Sync, we might as well use it then.
// We can test for using Rc&RefCell, but it's not really a significant overhead when comparing to
// refresh rate(even 240hz still needs 4.9ms, but the overhead from lock is only nanoseconds)
// pub struct WidgetPtr(Rc<RefCell<Widget>>);

pub struct SurfaceData {
    pub sctk: SctkSurfaceData,
    pub widget: AtomicPtr<std::sync::Weak<Mutex<Widget>>>,
}
impl SurfaceDataExt for SurfaceData {
    fn surface_data(&self) -> &SctkSurfaceData {
        &self.sctk
    }
}
impl SurfaceData {
    pub fn from_wl(wl: &WlSurface) -> &Self {
        wl.data::<SurfaceData>().unwrap()
    }
    fn store_widget(&self, widget: Weak<Mutex<Widget>>) {
        self.widget.store(
            Box::into_raw(Box::new(widget)),
            std::sync::atomic::Ordering::SeqCst,
        );
    }
    pub fn get_widget(&self) -> Option<Arc<Mutex<Widget>>> {
        unsafe {
            self.widget
                .load(std::sync::atomic::Ordering::SeqCst)
                .as_ref()
                .unwrap()
        }
        .upgrade()
    }
}
