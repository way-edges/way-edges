// NOTE: this widget is a mess
// we need better coding

use std::{cell::RefCell, rc::Rc};

use cairo::RectangleInt;
use display::grid::{BoxedWidgetRc, GridBox, GrideBoxBuilder};
use draw::DrawCore;
use event::event_handle;
use expose::{BoxExpose, BoxWidgetExpose};
use gio::glib::clone::Downgrade;
use gtk::glib;
use gtk::prelude::{DrawingAreaExtManual, GtkWindowExt, WidgetExt};
use gtk::DrawingArea;

use crate::config::widgets::wrapbox::BoxConfig;
use crate::config::Config;
use crate::ui::widgets::tray::init_tray;
use crate::ui::WidgetExposePtr;

use super::ring::init_ring;
use super::text::init_text;

pub mod display;
mod draw;
mod event;
pub mod expose;
pub mod outlook;

pub type MousePosition = (f64, f64);

type BoxCtxRc = Rc<RefCell<BoxCtx>>;

struct BoxCtx {
    // use
    input_region: RectangleInt,
    outlook: outlook::window::BoxOutlookWindow,
    pub grid_box: GridBox<BoxedWidgetRc>,
}

impl BoxCtx {
    fn new(config: &Config, box_conf: &mut BoxConfig, darea: &DrawingArea) -> (Self, BoxExpose) {
        // define box expose and create boxed widgets
        let expose = BoxExpose::new(darea);
        let mut grid_box = init_boxed_widgets(box_conf, expose.clone());

        // draw first frame
        // first draw
        let content = grid_box.draw_content();

        // create outlook
        let ol = match box_conf.outlook.take().unwrap() {
            crate::config::widgets::wrapbox::Outlook::Window(c) => {
                outlook::window::BoxOutlookWindow::new(
                    c,
                    (content.width(), content.height()),
                    config.edge,
                )
            }
        };

        (
            Self {
                input_region: RectangleInt::new(0, 0, 0, 0),
                outlook: ol,
                grid_box,
            },
            expose,
        )
    }

    fn update_input_region(&mut self, input_region: RectangleInt) {
        self.input_region = input_region;
    }
}

impl Drop for BoxCtx {
    fn drop(&mut self) {
        log::info!("drop box ctx");
    }
}

pub fn init_widget(
    window: &gtk::ApplicationWindow,
    conf: Config,
    mut box_conf: BoxConfig,
) -> Result<WidgetExposePtr, String> {
    let edge = conf.edge;
    let position = conf.position;
    let extra_trigger_size = box_conf.box_conf.extra_trigger_size.get_num_into().unwrap();

    let darea = {
        let darea = DrawingArea::new();
        window.set_child(Some(&darea));
        darea.connect_destroy(|_| {
            log::info!("destroy `box` drawing area");
        });
        darea
    };

    let (box_ctx, expose) = BoxCtx::new(&conf, &mut box_conf, &darea);
    let box_ctx = Rc::new(RefCell::new(box_ctx));

    let mut box_draw_core = DrawCore::new(
        &darea,
        &mut box_conf,
        box_ctx.clone(),
        &expose,
        edge,
        position,
        extra_trigger_size,
    );

    let widget_expose = {
        let box_motion_transition = box_draw_core.box_motion_transition.clone();
        let ms = event_handle(&darea, expose.clone(), box_motion_transition, box_ctx);
        Box::new(BoxWidgetExpose::new(ms.downgrade()))
    };

    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |darea, ctx, _, _| {
            box_draw_core.draw(ctx, darea, &window);
        }
    ));

    Ok(widget_expose)
}

fn init_boxed_widgets(box_conf: &mut BoxConfig, expose: BoxExpose) -> GridBox<BoxedWidgetRc> {
    let mut builder = GrideBoxBuilder::<BoxedWidgetRc>::new();
    let ws = std::mem::take(&mut box_conf.widgets);

    use crate::config::widgets::wrapbox::BoxedWidget;
    ws.into_iter().for_each(|w| {
        let _ = match w.widget {
            BoxedWidget::Ring(r) => match init_ring(&expose, *r) {
                Ok(ring) => {
                    builder.add(Rc::new(RefCell::new(ring)), (w.index[0], w.index[1]));
                    Ok(())
                }
                Err(e) => Err(format!("Fail to create ring widget: {e}")),
            },
            BoxedWidget::Text(t) => match init_text(&expose, *t) {
                Ok(text) => {
                    builder.add(Rc::new(RefCell::new(text)), (w.index[0], w.index[1]));
                    Ok(())
                }
                Err(e) => Err(format!("Fail to create text widget: {e}")),
            },
            BoxedWidget::Tray(c) => {
                let tray_ctx = init_tray(&expose, *c);
                builder.add(tray_ctx, (w.index[0], w.index[1]));
                Ok(())
            }
        }
        .inspect_err(|e| {
            crate::notify_send("Way-edges boxed widgets", e.as_str(), true);
            log::error!("{e}");
        });
    });

    builder.build(box_conf.box_conf.gap, box_conf.box_conf.align)
}
