use std::{
    cell::{Cell, UnsafeCell},
    collections::HashMap,
    rc::Rc,
    sync::{atomic::AtomicPtr, Arc, Mutex, MutexGuard, Weak},
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
    reexports::protocols::wp::fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::WpFractionalScaleV1,
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
        if let Some(mut w) = group.get_widget(wn) {
            w.toggle_pin(&self.queue_handle)
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
impl App {
    fn signal_redraw(&self, layer: &LayerSurface) {
        layer
            .wl_surface()
            .frame(&self.queue_handle, layer.wl_surface().clone());
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
    fn get_widget(&self, name: &str) -> Option<MutexGuard<Widget>> {
        self.widgets.get(name).map(|w| w.lock().unwrap())
    }
}

fn redraw(layer: &LayerSurface, qh: &QueueHandle<App>) {
    layer.wl_surface().frame(qh, layer.wl_surface().clone());
}

#[derive(Debug)]
pub struct Widget {
    pub name: String,
    pub monitor: MonitorSpecifier,
    pub configured: bool,

    pub output: WlOutput,
    pub layer: LayerSurface,
    pub scale: Scale,

    pub pop_animation: ToggleAnimationRc,
    pub animation_list: AnimationList,
    pub pop_animation_finished: bool,
    pub widget_animation_finished: bool,

    pub has_update: Rc<Cell<bool>>,
    pub mouse_state: MouseState,
    pub window_pop_state: WindowPopState,
    pub start_pos: (i32, i32),

    pub w: Box<dyn WidgetContext>,
    pub buffer: Buffer,
    pub width: i32,
    pub height: i32,
    pub draw_core: DrawCore,
}
impl Widget {
    fn needs_next_frame(&mut self) -> bool {
        let widget_has_animation_update = self.animation_list.has_in_progress;
        let pop_animation_update = self.pop_animation.borrow().is_in_progress();

        if widget_has_animation_update {
            if self.widget_animation_finished {
                self.widget_animation_finished = false
            }
            return true;
        } else if !self.widget_animation_finished {
            self.widget_animation_finished = true;
            return true;
        }

        if pop_animation_update {
            if self.pop_animation_finished {
                self.pop_animation_finished = false
            }
            return true;
        } else if !self.pop_animation_finished {
            self.pop_animation_finished = true;
            return true;
        }

        false
    }
    fn prepare_content(&mut self) {
        self.animation_list.refresh();
        self.pop_animation.borrow_mut().refresh();

        // update content
        let widget_has_animation_update = self.animation_list.has_in_progress;
        if self.has_update.get() || widget_has_animation_update {
            self.has_update.set(false);
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
        self.prepare_content();
        log::debug!("frame");

        // set size
        let (w, h) = self
            .scale
            .calculate_size(self.width as u32, self.height as u32);
        self.layer.set_size(w, h);

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

        self.layer.commit();

        // need next frame
        if self.needs_next_frame() {
            redraw(&self.layer, &app.queue_handle);
        }
    }

    fn toggle_pin(&mut self, qh: &QueueHandle<App>) {
        self.window_pop_state
            .toggle_pin(self.mouse_state.is_hovering());
        redraw(&self.layer, qh);
    }
    pub fn update_normal(&mut self, normal: u32, qh: &QueueHandle<App>) {
        if self.scale.update_normal(normal) {
            redraw(&self.layer, qh);
        }
    }
    pub fn update_fraction(&mut self, fraction: u32, qh: &QueueHandle<App>) {
        if self.scale.update_fraction(fraction) {
            redraw(&self.layer, qh);
        }
    }
    pub fn on_mouse_event(&mut self, qh: &QueueHandle<App>, event: &PointerEvent) {
        let Some(mut event) = self.mouse_state.from_wl_pointer(event) else {
            return;
        };

        log::debug!("pointer: {event:?}");

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
            self.has_update.set(true);
        }

        if trigger_redraw || widget_trigger_redraw {
            println!("trigger_redraw");
            redraw(&self.layer, qh);
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
    fraction: u32,
    fractional_client: Option<WpFractionalScaleV1>,
}
impl Scale {
    pub fn new(fractional_client: Option<WpFractionalScaleV1>) -> Self {
        Self {
            normal: 1,
            fraction: 0,
            fractional_client,
        }
    }
    pub fn update_normal(&mut self, normal: u32) -> bool {
        let changed = self.normal != normal;
        self.normal = normal;
        changed
    }
    pub fn update_fraction(&mut self, fraction: u32) -> bool {
        let changed = self.fraction != fraction;
        self.fraction = fraction;
        changed
    }
    pub fn calculate_size(&self, width: u32, height: u32) -> (u32, u32) {
        if self.fractional_client.is_some() && self.fraction != 0 {
            (
                (width * self.fraction + 60) / 120,
                (height * self.fraction + 60) / 120,
            )
        } else {
            (width / self.normal, height / self.normal)
        }
    }
}

struct PopEssential {
    pop_animation: ToggleAnimationRcWeak,
    pop_state: std::rc::Weak<UnsafeCell<Option<Rc<()>>>>,
    pop_duration: Duration,
    layer: LayerSurface,
}
impl PopEssential {
    fn pop(&self, app: &App) {
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
            app.signal_redraw(&self.layer);

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
                    app.signal_redraw(&layer);

                    calloop::timer::TimeoutAction::Drop
                },
            )
            .unwrap();
    }
}

struct RedrawEssentail {
    has_update: std::rc::Weak<Cell<bool>>,
    layer: LayerSurface,
}
impl RedrawEssentail {
    fn redraw(&self, app: &App) {
        let Some(has_update) = self.has_update.upgrade() else {
            return;
        };
        has_update.set(true);
        // signal redraw
        app.signal_redraw(&self.layer);
    }
}

pub struct WidgetBuilder<'a> {
    pub name: String,
    pub monitor: MonitorSpecifier,
    pub output: WlOutput,
    pub app: &'a App,
    pub layer: LayerSurface,
    pub scale: Scale,

    pub has_update: Rc<Cell<bool>>,

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
        let has_update = Rc::downgrade(&self.has_update);
        let layer = self.layer.clone();
        RedrawEssentail { has_update, layer }
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
            });
        let scale = Scale::new(fractional);

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
            has_update: Rc::new(Cell::new(true)),
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
            has_update,
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
            has_update,
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
