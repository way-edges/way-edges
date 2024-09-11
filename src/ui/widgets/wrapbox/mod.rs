// NOTE: this widget is a mess
// we need better coding

use std::{cell::RefCell, rc::Rc};

use async_channel::Receiver;
use cairo::RectangleInt;
use display::grid::{GridBox, GridItemSizeMap};
use draw::DrawCore;
use event::event_handle;
use expose::{BoxExpose, BoxExposeRc, BoxWidgetExpose};
use gio::glib::clone::Downgrade;
use gtk::glib;
use gtk::prelude::{DrawingAreaExtManual, GtkWindowExt, WidgetExt};
use gtk::DrawingArea;

use crate::config::widgets::wrapbox::{BoxConfig, BoxedWidgetConfig};
use crate::config::Config;
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
    item_map: GridItemSizeMap,
    rec_int: RectangleInt,
    outlook: outlook::window::BoxOutlookWindow,
    grid_box: GridBox,
}

impl BoxCtx {
    fn new(config: &Config, box_conf: &mut BoxConfig) -> (Self, Receiver<()>, BoxExposeRc) {
        let mut grid_box =
            display::grid::GridBox::new(box_conf.box_conf.gap, box_conf.box_conf.align);

        // define box expose and create boxed widgets
        let (expose, update_signal_receiver) = BoxExpose::new();
        init_boxed_widgets(
            &mut grid_box,
            expose.clone(),
            std::mem::take(&mut box_conf.widgets),
        );

        // draw first frame
        // first draw
        let (content, item_map) = grid_box.draw_content();

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
                item_map,
                rec_int: RectangleInt::new(0, 0, 0, 0),
                outlook: ol,
                grid_box,
            },
            update_signal_receiver,
            expose,
        )
    }

    fn update_box_ctx(&mut self, item_size_map: GridItemSizeMap, rec_int: RectangleInt) {
        self.item_map = item_size_map;
        self.rec_int = rec_int;
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
    let position = conf.position.unwrap();
    let extra_trigger_size = box_conf.box_conf.extra_trigger_size.get_num_into().unwrap();

    let darea = DrawingArea::new();
    window.set_child(Some(&darea));
    darea.connect_destroy(|_| {
        log::info!("destroy `box` drawing area");
    });

    // let (box_ctx, update_signal_receiver, expose, box_motion_transition) = BoxCtx::new(
    let (box_ctx, update_signal_receiver, expose) = BoxCtx::new(&conf, &mut box_conf);
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
    let box_motion_transition = box_draw_core.box_motion_transition.clone();

    // it's a async block once, doesn't matter strong or weak
    glib::spawn_future_local(glib::clone!(
        #[weak]
        darea,
        async move {
            log::debug!("box draw signal receive loop start");
            while (update_signal_receiver.recv().await).is_ok() {
                darea.queue_draw();
            }
            log::debug!("box draw signal receive loop exit");
        }
    ));

    darea.set_draw_func(glib::clone!(
        #[weak]
        window,
        move |darea, ctx, _, _| {
            box_draw_core.draw(ctx, darea, &window);
        }
    ));

    let ms = event_handle(&darea, expose.clone(), box_motion_transition, box_ctx);
    Ok(Box::new(BoxWidgetExpose::new(ms.downgrade(), expose)))
}

fn init_boxed_widgets(bx: &mut GridBox, expose: BoxExposeRc, ws: Vec<BoxedWidgetConfig>) {
    ws.into_iter().for_each(|w| {
        let _ = match w.widget {
            crate::config::Widget::Ring(r) => match init_ring(&expose, *r) {
                Ok(ring) => {
                    bx.add(Rc::new(RefCell::new(ring)), (w.index[0], w.index[1]));
                    Ok(())
                }
                Err(e) => Err(format!("Fail to create ring widget: {e}")),
            },
            crate::config::Widget::Text(t) => match init_text(&expose, *t) {
                Ok(text) => {
                    bx.add(Rc::new(RefCell::new(text)), (w.index[0], w.index[1]));
                    Ok(())
                }
                Err(e) => Err(format!("Fail to create text widget: {e}")),
            },
            _ => unreachable!(),
        }
        .inspect_err(|e| {
            crate::notify_send("Way-edges boxed widgets", e.as_str(), true);
            log::error!("{e}");
        });
    });
}
