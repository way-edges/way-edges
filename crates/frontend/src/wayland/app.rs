use std::{
    cell::UnsafeCell,
    collections::HashMap,
    rc::Rc,
    sync::{atomic::AtomicPtr, Arc, Mutex, Weak},
    time::Duration,
};

use backend::ipc::IPCCommand;
use calloop::{
    channel::Sender,
    ping::{make_ping, Ping},
    LoopHandle, LoopSignal,
};
use config::MonitorSpecifier;
use glib::clone::{Downgrade, Upgrade};
use smithay_client_toolkit::{
    compositor::{CompositorState, Region, SurfaceData as SctkSurfaceData, SurfaceDataExt},
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
use util::Z;
use wayland_client::{
    protocol::{wl_output::WlOutput, wl_pointer, wl_surface::WlSurface},
    Proxy, QueueHandle,
};

use crate::{
    animation::{AnimationList, ToggleAnimation, ToggleAnimationRc, ToggleAnimationRcWeak},
    buffer::Buffer,
    mouse_state::{MouseEvent, MouseState},
    widgets::{init_widget, WidgetContext},
};

use super::{draw::DrawCore, window_pop_state::WindowPopState};

pub struct App {
    pub exit: bool,
    pub groups: HashMap<String, Option<Group>>,

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
}
impl App {
    pub fn handle_ipc(&mut self, cmd: IPCCommand) {
        match cmd {
            IPCCommand::AddGroup(s) => self.add_group(&s),
            IPCCommand::RemoveGroup(s) => self.rm_group(&s),
            IPCCommand::TogglePin(gn, wn) => self.toggle_pin(&gn, &wn),
            IPCCommand::Exit => self.exit = true,
        };
    }

    fn add_group(&mut self, name: &str) {
        if self.groups.contains_key(name) {
            return;
        }
        let group = self.init_group(name);
        self.groups.insert(name.to_string(), group);
    }
    fn rm_group(&mut self, name: &str) {
        drop(self.groups.remove(name));
    }
    fn toggle_pin(&mut self, gn: &str, wn: &str) {
        let Some(Some(group)) = self.groups.get_mut(gn) else {
            return;
        };
        if let Some(w) = group.get_widget(wn) {
            w.lock().unwrap().toggle_pin(self)
        }
    }

    pub fn reload(&mut self) {
        // FIX: HOW CAN WE DO THIS???
        let ptr = self as *const App;
        for (k, widget_map) in self.groups.iter_mut() {
            drop(widget_map.take());
            *widget_map = unsafe { ptr.as_ref().unwrap() }.init_group(k.as_str());
        }
    }

    fn init_group(&self, name: &str) -> Option<Group> {
        let conf = config::get_config_by_group(Some(name));
        let res = conf.and_then(|vc| {
            let Some(vc) = vc else {
                return Err(format!("Not found config by group: {name}"));
            };
            log::debug!("group config:\n{vc:?}");
            Group::init_group(vc.widgets, self)
        });
        res.inspect_err(|e| {
            log::error!("{e}");
            util::notify_send("Way-edges app error", e, true);
        })
        .ok()
    }
}

pub struct Group {
    pub widgets: HashMap<String, Arc<Mutex<Widget>>>,
}
impl Group {
    fn init_group(widgets_config: Vec<config::Config>, app: &App) -> Result<Self, String> {
        let widgets = widgets_config
            .into_iter()
            .map(|cfg| {
                let widget_name = cfg.name.clone();
                let window_ctx = Widget::init_widget(cfg, app)?;
                Ok((widget_name, window_ctx))
            })
            .collect::<Result<HashMap<String, Arc<Mutex<Widget>>>, String>>()?;

        Ok(Self { widgets })
    }
    fn get_widget(&self, name: &str) -> Option<Arc<Mutex<Widget>>> {
        self.widgets.get(name).cloned()
    }
}

#[derive(Debug)]
pub struct Widget {
    pub name: String,
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
    pub width: i32,
    pub height: i32,
    pub draw_core: DrawCore,

    pub pop_animation: ToggleAnimationRc,
    pub animation_list: AnimationList,

    pop_animation_finished: bool,
    widget_animation_finished: bool,

    widget_has_update: bool,
    next_frame: bool,
    frame_available: bool,
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
            let size = self.draw_core.calc_max_size((img.width(), img.height()));
            self.width = size.0;
            self.height = size.1;
            self.buffer.update_buffer(img);
        }
    }
    fn draw_content(&mut self, ctx: &cairo::Context) -> [i32; 4] {
        // prepare pop
        let content = self.buffer.get_buffer();
        let content_size = (content.width(), content.height());
        let area_size = (self.width, self.height);
        let progress = self.pop_animation.borrow_mut().progress();

        // translate pop
        let pose = self
            .draw_core
            .draw_pop(ctx, area_size, content_size, progress);
        self.start_pos = (pose[0], pose[1]);

        ctx.set_source_surface(content, Z, Z).unwrap();
        ctx.paint().unwrap();
        pose
    }
    pub fn draw(&mut self, app: &mut App) {
        if self.next_frame {
            self.next_frame = false
        }
        self.prepare_content();
        log::debug!("frame");

        // create and draw content
        let (buffer, canvas) = app
            .pool
            .create_buffer(
                self.width,
                self.height,
                self.width * 4,
                wayland_client::protocol::wl_shm::Format::Argb8888,
            )
            .unwrap();
        buffer
            .attach_to(self.layer.wl_surface())
            .expect("buffer attach");
        // clear old buffer*
        canvas.iter_mut().for_each(|i| {
            *i = 0;
        });
        let surf = unsafe {
            cairo::ImageSurface::create_for_data_unsafe(
                canvas.as_mut_ptr(),
                cairo::Format::ARgb32,
                self.width,
                self.height,
                self.width * 4,
            )
            .unwrap()
        };
        let ctx = cairo::Context::new(&surf).unwrap();
        let pose = self.draw_content(&ctx);

        // attach content
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, self.width, self.height);

        // set input region
        let input_rect = self.draw_core.calc_input_region(pose);
        let r = Region::new(&app.compositor_state).unwrap();
        r.add(input_rect[0], input_rect[1], input_rect[2], input_rect[3]);
        self.layer.set_input_region(Some(r.wl_region()));

        // set size
        let (w, h) = self
            .scale
            .calculate_size(self.width as u32, self.height as u32);
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
    }
    pub fn update_fraction(&mut self, fraction: u32, app: &mut App) {
        if self.scale.update_fraction(fraction) {
            self.try_redraw(app);
        }
    }
    pub fn on_mouse_event(&mut self, app: &mut App, event: &PointerEvent) {
        let Some(mut event) = self.mouse_state.from_wl_pointer(event) else {
            return;
        };

        // log::debug!("pointer: {event:?}");

        let data = &mut self.mouse_state.data;

        let mut trigger_redraw = false;
        let mut do_redraw = || {
            if !trigger_redraw {
                trigger_redraw = true;
            }
        };

        fn change_pos(pose: &mut (f64, f64), start_pose: (i32, i32)) {
            pose.0 -= start_pose.0 as f64;
            pose.1 -= start_pose.1 as f64;
        }

        match &mut event {
            MouseEvent::Release(pos, _) | MouseEvent::Press(pos, _) => {
                change_pos(pos, self.start_pos)
            }
            MouseEvent::Enter(pos) | MouseEvent::Motion(pos) => change_pos(pos, self.start_pos),
            MouseEvent::Leave => {}
        }

        match event {
            MouseEvent::Release(_, key) => {
                if key == self.window_pop_state.pin_key {
                    self.window_pop_state.toggle_pin(data.hovering);
                    do_redraw()
                };
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

    fn init_widget(mut conf: config::Config, app: &App) -> Result<Arc<Mutex<Self>>, String> {
        let mut builder = WidgetBuilder::new(&mut conf, app)?;
        let w = init_widget(&mut conf, &mut builder);
        let s = builder.build(conf, w);

        Ok(Arc::new_cyclic(|weak| {
            SurfaceData::from_wl(s.layer.wl_surface()).store_widget(weak.clone());
            Mutex::new(s)
        }))
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
            let size = ((width * 120 + 60) / scale, (height * 120 + 60) / scale);

            // viewport
            fractional.2.set_destination(size.0 as i32, size.1 as i32);
        }

        (width / self.normal, height / self.normal)
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

struct PopEssential {
    pop_animation: ToggleAnimationRcWeak,
    pop_state: std::rc::Weak<UnsafeCell<Option<Rc<()>>>>,
    pop_duration: Duration,
    layer: LayerSurface,
}
impl PopEssential {
    fn signal_pop_redraw(layer: &LayerSurface, app: &mut App) {
        let Some(w) = SurfaceData::from_wl(layer.wl_surface()).get_widget() else {
            return;
        };
        w.lock().unwrap().try_redraw(app);
    }
    fn pop(&self, app: &mut App) {
        // pop up
        let guard_weak = {
            let Some(pop_animation) = self.pop_animation.upgrade() else {
                return;
            };
            let Some(pop_state) = self.pop_state.upgrade() else {
                return;
            };

            let guard = Rc::new(());
            let guard_weak = Rc::downgrade(&guard);
            unsafe { pop_state.get().as_mut().unwrap().replace(guard) };

            pop_animation
                .borrow_mut()
                .set_direction(crate::animation::ToggleDirection::Forward);
            Self::signal_pop_redraw(&self.layer, app);

            guard_weak
        };

        // hide
        let layer = self.layer.clone();
        let pop_animation = self.pop_animation.clone();
        let pop_duration = self.pop_duration;
        app.event_loop_handle
            .insert_source(
                calloop::timer::Timer::from_duration(pop_duration),
                move |_, _, app| {
                    let Some(pop_animation) = pop_animation.upgrade() else {
                        return calloop::timer::TimeoutAction::Drop;
                    };
                    if guard_weak.upgrade().is_none() {
                        return calloop::timer::TimeoutAction::Drop;
                    }

                    pop_animation
                        .borrow_mut()
                        .set_direction(crate::animation::ToggleDirection::Backward);
                    Self::signal_pop_redraw(&layer, app);

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
    pub name: String,
    pub monitor: MonitorSpecifier,
    pub output: WlOutput,
    pub app: &'a App,
    pub layer: LayerSurface,
    pub scale: Scale,

    pub pop_animation: ToggleAnimationRc,
    pub animation_list: AnimationList,
    pub pop_state: Rc<UnsafeCell<Option<Rc<()>>>>,
}
impl WidgetBuilder<'_> {
    pub fn new_animation(&mut self, time_cost: u64) -> ToggleAnimationRc {
        self.animation_list.new_transition(time_cost)
    }
    pub fn extend_animation_list(&mut self, list: &AnimationList) {
        self.animation_list.extend_list(list);
    }
    fn make_pop_essential(&self, pop_duration: u64) -> PopEssential {
        let layer = self.layer.clone();
        let pop_animation = self.pop_animation.downgrade();
        let pop_state = Rc::downgrade(&self.pop_state);
        let pop_duration = Duration::from_millis(pop_duration);
        PopEssential {
            pop_animation,
            pop_state,
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
    fn new(conf: &mut config::Config, app: &'a App) -> Result<WidgetBuilder<'a>, String> {
        let output = match &conf.monitor {
            MonitorSpecifier::ID(index) => app.output_state.outputs().nth(*index),
            MonitorSpecifier::Name(name) => app.output_state.outputs().find(|out| {
                app.output_state
                    .info(out)
                    .and_then(|info| info.name)
                    .filter(|output_name| output_name == name)
                    .is_some()
            }),
        }
        .ok_or(format!("output not found: {:?}", conf.monitor))?;
        let monitor = app.output_state.info(&output).unwrap();
        let size = monitor.modes[0].dimensions;
        conf.resolve_relative(size);

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
            conf.layer,
            Some("way-edges-widget"),
            Some(&output),
        );
        layer.set_anchor(conf.edge | conf.position);
        if conf.ignore_exclusive {
            layer.set_exclusive_zone(-1);
        };
        layer.set_margin(
            conf.margins.top.get_num().unwrap() as i32,
            conf.margins.right.get_num().unwrap() as i32,
            conf.margins.bottom.get_num().unwrap() as i32,
            conf.margins.left.get_num().unwrap() as i32,
        );
        layer.set_size(1, 1);
        layer.commit();

        let pop_animation = ToggleAnimation::new(
            Duration::from_millis(conf.transition_duration),
            crate::animation::Curve::Linear,
        )
        .make_rc();
        let pop_state = Rc::new(UnsafeCell::new(None));
        let animation_list = AnimationList::new();

        Ok(Self {
            name: conf.name.clone(),
            monitor: conf.monitor.clone(),
            output,
            app,
            layer,
            pop_animation,
            animation_list,
            pop_state,
            scale,
        })
    }
    pub fn build(self, conf: config::Config, w: Box<dyn WidgetContext>) -> Widget {
        let Self {
            name,
            monitor,
            output,
            app: _,
            layer,
            scale,
            pop_animation,
            animation_list,
            pop_state,
        } = self;

        let start_pos = (0, 0);
        let mouse_state = MouseState::new();
        let window_pop_state = WindowPopState::new(pop_animation.clone(), pop_state);
        let buffer = Buffer::default();
        let draw_core = DrawCore::new(&conf);

        Widget {
            name,
            monitor,
            configured: false,
            output,
            layer,
            scale,
            pop_animation,
            animation_list,
            mouse_state,
            window_pop_state,
            start_pos,
            w,
            buffer,
            draw_core,
            pop_animation_finished: true,
            widget_animation_finished: true,
            width: 1,
            height: 1,
            widget_has_update: true,
            next_frame: false,
            frame_available: true,
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
