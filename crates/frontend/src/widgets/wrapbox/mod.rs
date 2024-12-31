mod box_traits;
mod event;
mod grid;
mod outlook;
mod widgets;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{
    animation::{AnimationList, ToggleAnimationRc},
    window::WindowContext,
};
use box_traits::{BoxedWidgetCtx, BoxedWidgetCtxRc, BoxedWidgetGrid};
use config::{widgets::wrapbox::BoxConfig, Config};
use grid::builder::GrideBoxBuilder;
use gtk::{gdk::Monitor, glib};
use outlook::init_outlook;

pub fn init_widget(window: &mut WindowContext, _: &Monitor, conf: Config, mut w_conf: BoxConfig) {
    let grid_box = Rc::new(RefCell::new(init_boxed_widgets(window, &mut w_conf)));

    let (outlook_mouse_pos, draw_outlook) = init_outlook(w_conf.outlook, &conf);

    window.set_draw_func(Some(glib::clone!(
        #[strong]
        grid_box,
        move || {
            let start = std::time::Instant::now();

            let content = grid_box.borrow_mut().redraw_if_has_update()?;
            let img = draw_outlook(content);

            println!("cost: {}ms", start.elapsed().as_secs_f64() * 1000.);
            Some(img)
        }
    )));

    event::event_handle(window, &grid_box, outlook_mouse_pos);
}

fn init_boxed_widgets(window: &mut WindowContext, box_conf: &mut BoxConfig) -> BoxedWidgetGrid {
    let mut builder = GrideBoxBuilder::<BoxedWidgetCtxRc>::new();
    let ws = std::mem::take(&mut box_conf.widgets);

    use config::widgets::wrapbox::BoxedWidget;
    ws.into_iter().for_each(|w| {
        let mut box_temporary_ctx = BoxTemporaryCtx::new(window);

        let widget = match w.widget {
            BoxedWidget::Text(text_config) => {
                widgets::text::init_text(&mut box_temporary_ctx, text_config)
            }
            BoxedWidget::Ring(ring_config) => todo!(),
            BoxedWidget::Tray(tray_config) => todo!(),
        };

        let boxed_widget_context = box_temporary_ctx.to_boxed_widget_ctx(widget).make_rc();
        builder.add(boxed_widget_context, (w.index[0], w.index[1]));
    });

    builder.build(box_conf.gap, box_conf.align)
}

struct BoxTemporaryCtx<'a> {
    window: &'a mut WindowContext,
    animation_list: AnimationList,
    has_update: Rc<Cell<bool>>,
}
impl<'a> BoxTemporaryCtx<'a> {
    fn new(window: &'a mut WindowContext) -> Self {
        Self {
            window,
            animation_list: AnimationList::new(),
            has_update: Rc::new(Cell::new(false)),
        }
    }
    fn new_animation(&mut self, time_cost: u64) -> ToggleAnimationRc {
        self.animation_list.new_transition(time_cost)
    }
    fn make_redraw_signal(&mut self) -> impl Fn() {
        let func = self.window.make_redraw_notifier();
        let has_update = &self.has_update;
        glib::clone!(
            #[weak]
            has_update,
            move || {
                has_update.set(true);
                func(None)
            }
        )
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_boxed_widget_ctx(self, ctx: impl box_traits::BoxedWidget + 'static) -> BoxedWidgetCtx {
        self.window.extend_animation_list(&self.animation_list);
        BoxedWidgetCtx::new(ctx, self.animation_list, self.has_update)
    }
}
