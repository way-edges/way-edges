use std::{
    cell::{Cell, RefCell, UnsafeCell},
    rc::Rc,
    time::Duration,
};

use cairo::ImageSurface;
use config::{Config, MonitorSpecifier};
use gtk::{
    gdk::Monitor,
    prelude::{GtkWindowExt, WidgetExt},
    Application, ApplicationWindow, CssProvider, DrawingArea,
};
use gtk4_layer_shell::LayerShell;

use crate::{
    animation::{AnimationList, ToggleAnimation, ToggleAnimationRc},
    buffer::Buffer,
    mouse_state::{MouseEvent, MouseState, MouseStateData},
};

use super::{
    draw::{make_base_draw_func, make_max_size_func, set_draw_func},
    event::{setup_mouse_event_callback, WindowPopState},
    frame::WindowFrameManager,
    WindowContext,
};

pub trait WidgetContext {
    fn redraw(&mut self) -> ImageSurface;
    fn on_mouse_event(&mut self, data: &MouseStateData, event: MouseEvent) -> bool;
    fn make_rc(self) -> Rc<RefCell<dyn WidgetContext>>
    where
        Self: Sized + 'static,
    {
        Rc::new(RefCell::new(self))
    }
}

type PopStateGuard = Rc<()>;

// NOTE: THIS CAN BE MODIFIED ANYTIME WHEN NEEDED
pub struct WindowContextBuilder {
    name: String,
    monitor: MonitorSpecifier,
    window: ApplicationWindow,
    drawing_area: DrawingArea,

    has_update: Rc<Cell<bool>>,

    pop_animation: ToggleAnimationRc,
    animation_list: AnimationList,
    redraw_rc: Rc<dyn Fn()>,
    pop_state: Rc<UnsafeCell<Option<PopStateGuard>>>,
    pop_duration: Duration,
}
impl WindowContextBuilder {
    pub fn new_animation(&mut self, time_cost: u64) -> ToggleAnimationRc {
        self.animation_list.new_transition(time_cost)
    }
    pub fn extend_animation_list(&mut self, list: &AnimationList) {
        self.animation_list.extend_list(list);
    }
    pub fn make_pop_func(&mut self) -> impl Fn() {
        let signal_redraw = Rc::downgrade(&self.redraw_rc);
        let pop_animation = &self.pop_animation;
        let pop_state = &self.pop_state;
        let pop_duration = self.pop_duration;

        use gtk::glib;
        glib::clone!(
            #[weak]
            pop_animation,
            #[weak]
            pop_state,
            move || {
                let Some(signal_redraw) = signal_redraw.upgrade() else {
                    return;
                };

                let guard = Rc::new(());
                let guard_weak = Rc::downgrade(&guard);
                unsafe { pop_state.get().as_mut().unwrap().replace(guard) };

                pop_animation
                    .borrow_mut()
                    .set_direction(crate::animation::ToggleDirection::Forward);
                signal_redraw();

                glib::timeout_add_local_once(pop_duration, move || {
                    if guard_weak.upgrade().is_none() {
                        return;
                    }
                    pop_animation
                        .borrow_mut()
                        .set_direction(crate::animation::ToggleDirection::Backward);
                    signal_redraw()
                });
            }
        )
    }
    pub fn make_redraw_notifier(&self) -> impl Fn() {
        let drawing_area = &self.drawing_area;
        let has_update = &self.has_update;
        glib::clone!(
            #[weak]
            drawing_area,
            #[weak]
            has_update,
            move || {
                has_update.set(true);
                drawing_area.queue_draw();
            }
        )
    }
}

impl WindowContextBuilder {
    /// config and monitor should be ready before this
    pub fn new(app: &Application, monitor: &Monitor, conf: &Config) -> Result<Self, String> {
        let window = gtk::ApplicationWindow::new(app);

        // init layer
        window.init_layer_shell();
        window.set_monitor(monitor);
        window.set_layer(conf.layer);
        window.set_namespace("way-edges-widget");

        // edge and position
        window.set_anchor(conf.edge, true);
        window.set_anchor(conf.position, true);

        if conf.ignore_exclusive {
            window.set_exclusive_zone(-1);
        }

        conf.margins
            .iter()
            .try_for_each(|(e, m)| -> Result<(), String> {
                window.set_margin(*e, m.get_num_into()? as i32);
                Ok(())
            })?;

        // set something after show
        window.connect_show(|w: &gtk::ApplicationWindow| {
            // transparency background !! may not work for some gtk4 theme, and idk what to do with it !!
            let provider = CssProvider::new();
            provider
                // .load_from_string("window.background { background: unset; border: 1px solid white; }");
                .load_from_string("window.background { background: unset; }");
            gtk::style_context_add_provider_for_display(
                &WidgetExt::display(w),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });

        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(1, 1);
        window.set_child(Some(&drawing_area));

        let pop_animation = ToggleAnimation::new(
            Duration::from_millis(conf.transition_duration),
            crate::animation::Curve::Linear,
        )
        .make_rc();
        let pop_state = Rc::new(UnsafeCell::new(None));
        let pop_duration = Duration::from_millis(conf.transition_duration);

        let animation_list = AnimationList::new();

        let redraw_rc = Rc::new(glib::clone!(
            #[weak]
            drawing_area,
            move || {
                drawing_area.queue_draw();
            }
        ));

        Ok(Self {
            name: conf.name.clone(),
            monitor: conf.monitor.clone(),
            window,
            drawing_area,
            animation_list,
            pop_animation,
            pop_state,
            redraw_rc,
            pop_duration,
            has_update: Rc::new(Cell::new(true)),
        })
    }
    pub fn build(self, conf: Config, widget: Rc<RefCell<dyn WidgetContext>>) -> WindowContext {
        let Self {
            name,
            monitor,
            window,
            drawing_area,
            has_update,
            pop_animation,
            animation_list,
            redraw_rc: _,
            pop_state,
            pop_duration: _,
        } = self;

        let start_pos = Rc::new(Cell::new((0, 0)));
        let mouse_state = MouseState::new().connect(&drawing_area);
        let window_pop_state = WindowPopState::new(pop_animation.clone(), pop_state).make_rc();

        // draw
        {
            let frame_manager = WindowFrameManager::new(
                conf.frame_rate.unwrap() as u64,
                animation_list,
                pop_animation.clone(),
            );
            let buffer = Buffer::default();
            let base_draw_func = make_base_draw_func(&conf);
            let max_size_func = make_max_size_func(
                conf.edge,
                conf.extra_trigger_size.get_num_into().unwrap().ceil() as i32,
            );
            let widget = Rc::downgrade(&widget);

            set_draw_func(
                &drawing_area,
                &window,
                &start_pos,
                &pop_animation,
                widget,
                has_update,
                frame_manager,
                buffer,
                base_draw_func,
                max_size_func,
            );
        };

        // event
        {
            let widget = Rc::downgrade(&widget);
            setup_mouse_event_callback(
                &drawing_area,
                &start_pos,
                &mouse_state,
                &window_pop_state,
                widget,
            );
        };

        WindowContext {
            name,
            monitor,
            window,
            drawing_area,
            start_pos,
            mouse_state,
            window_pop_state,
            widget,
        }
    }
}
