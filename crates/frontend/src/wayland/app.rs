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
    compositor::{CompositorState, SurfaceData as SctkSurfaceData, SurfaceDataExt},
    output::OutputState,
    reexports::protocols::wp::fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::WpFractionalScaleV1,
    },
    registry::{GlobalProxy, RegistryState},
    seat::SeatState,
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

use crate::animation::{AnimationList, ToggleAnimation, ToggleAnimationRc, ToggleAnimationRcWeak};

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
            w.toggle_pin()
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

pub struct Widget {
    pub configured: bool,
    pub layer: LayerSurface,
    pub scale: Scale,
}
impl Widget {
    fn toggle_pin(&mut self) {
        todo!()
    }
    pub fn update_normal(&mut self, normal: u32) {
        self.scale.update_normal(normal)
    }
    pub fn update_fraction(&mut self, fraction: u32) {
        self.scale.update_fraction(fraction)
    }
    fn init_widget(conf: config::Config, app: &App) -> Result<Arc<Mutex<Self>>, String> {
        let builder = WidgetBuilder::new(&conf, app)?;

        // Arc::new_cyclic(|weak| {
        //     SurfaceData::from_wl()
        //     Mutex::new(s)
        // })

        todo!()
    }
}

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
    pub fn update_normal(&mut self, normal: u32) {
        self.normal = normal;
    }
    pub fn update_fraction(&mut self, fraction: u32) {
        self.fraction = fraction;
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
        app.event_loop_handle.insert_source(
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
        );
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
impl<'a> WidgetBuilder<'a> {
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
            });

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
            });

        ping
    }
    pub fn make_pop_ping(&mut self, pop_duration: u64) -> Ping {
        let (ping, source) = make_ping().unwrap();

        let pop_essential = self.make_pop_essential(pop_duration);
        self.app
            .event_loop_handle
            .insert_source(source, move |_, _, app| {
                pop_essential.pop(app);
            });

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
            });

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
            });

        ping
    }
    pub fn make_redraw_ping(&self) -> Ping {
        let (ping, source) = make_ping().unwrap();

        let redraw_essential = self.make_redraw_essentail();
        self.app
            .event_loop_handle
            .insert_source(source, move |_, _, app| {
                redraw_essential.redraw(app);
            });

        ping
    }
}
impl<'a> WidgetBuilder<'a> {
    fn new(conf: &config::Config, app: &'a App) -> Result<WidgetBuilder<'a>, String> {
        let mut outputs = app.output_state.outputs();
        let output = match &conf.monitor {
            MonitorSpecifier::ID(index) => outputs.nth(*index),
            MonitorSpecifier::Name(name) => outputs.find(|out| {
                app.output_state
                    .info(out)
                    .and_then(|info| info.name)
                    .map(|output_name| &output_name == name)
                    .is_some()
            }),
        }
        .ok_or(format!("output not found: {:?}", conf.monitor))?;

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
