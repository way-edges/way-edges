mod box_traits;
mod event;
mod grid;
mod outlook;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::window::WindowContext;
use box_traits::{BoxedWidgetCtxRc, BoxedWidgetGrid};
use config::{widgets::wrapbox::BoxConfig, Config};
use grid::builder::GrideBoxBuilder;
use gtk::{gdk::Monitor, glib};
use outlook::init_outlook;

pub fn init_widget(window: &mut WindowContext, _: &Monitor, conf: Config, mut w_conf: BoxConfig) {
    let widget_builder = BoxWidgetBuilder::new(window);
    let has_update = widget_builder.has_update.clone();
    let grid_box = Rc::new(RefCell::new(init_boxed_widgets(
        &mut w_conf,
        widget_builder,
    )));

    let (outlook_mouse_pos, draw_outlook) = init_outlook(w_conf.outlook, &conf);

    window.set_draw_func(Some(glib::clone!(
        #[strong]
        grid_box,
        move || {
            if !has_update.get() {
                return None;
            }

            let content = grid_box.borrow_mut().draw();
            let img = draw_outlook(content);
            Some(img)
        }
    )));

    event::event_handle(window, &grid_box, outlook_mouse_pos);
}

struct BoxWidgetBuilder<'a> {
    window: &'a mut WindowContext,
    has_update: Rc<Cell<bool>>,
}
impl<'a> BoxWidgetBuilder<'a> {
    fn new(window: &'a mut WindowContext) -> Self {
        Self {
            window,
            has_update: Rc::new(Cell::new(false)),
        }
    }
    fn make_redraw_notifier(&self) -> impl Fn() {
        let window_redraw = self.window.make_redraw_notifier();
        use gtk::glib;
        let is_updated = &self.has_update;
        glib::clone!(
            #[weak]
            is_updated,
            move || {
                is_updated.set(true);
                window_redraw(None)
            }
        )
    }
}

fn init_boxed_widgets(box_conf: &mut BoxConfig, builder: BoxWidgetBuilder) -> BoxedWidgetGrid {
    let mut builder = GrideBoxBuilder::<BoxedWidgetCtxRc>::new();
    let ws = std::mem::take(&mut box_conf.widgets);

    use config::widgets::wrapbox::BoxedWidget;
    ws.into_iter().for_each(|w| {
        let widget = match w.widget {
            BoxedWidget::Ring(ring_config) => todo!(),
            BoxedWidget::Text(text_config) => todo!(),
            BoxedWidget::Tray(tray_config) => todo!(),
        };

        builder.add(Rc::new(RefCell::new(widget)), (w.index[0], w.index[1]));
    });

    builder.build(box_conf.gap, box_conf.align)
}
