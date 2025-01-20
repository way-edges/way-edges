mod box_traits;
mod event;
mod grid;
mod outlook;
mod widgets;

use std::{cell::Cell, rc::Rc};

use crate::{
    animation::{AnimationList, ToggleAnimationRc},
    wayland::app::{App, WidgetBuilder},
};
use box_traits::{BoxedWidgetCtx, BoxedWidgetGrid};
use config::{widgets::wrapbox::BoxConfig, Config};
use event::LastWidget;
use grid::{builder::GrideBoxBuilder, GridBox};
use outlook::{init_outlook, OutlookDrawConf};

use super::WidgetContext;

pub struct BoxContext {
    grid_box: GridBox<BoxedWidgetCtx>,
    outlook_draw_conf: OutlookDrawConf,

    last_widget: LastWidget,
    leave_box_state: bool,
}
impl WidgetContext for BoxContext {
    fn redraw(&mut self) -> cairo::ImageSurface {
        let content = self.grid_box.draw();
        self.outlook_draw_conf.draw(content)
    }

    fn on_mouse_event(
        &mut self,
        _: &crate::mouse_state::MouseStateData,
        event: crate::mouse_state::MouseEvent,
    ) -> bool {
        event::on_mouse_event(event, self)
    }
}

pub fn init_widget(
    window: &mut WidgetBuilder,
    conf: &Config,
    mut w_conf: BoxConfig,
) -> impl WidgetContext {
    let grid_box = init_boxed_widgets(window, &mut w_conf);
    let outlook_draw_conf = init_outlook(w_conf.outlook, conf);

    BoxContext {
        grid_box,
        outlook_draw_conf,
        // last hover widget, for trigger mouse leave option for that widget.
        last_widget: LastWidget::new(),
        // because mouse leave event is before release,
        // we need to check if unpress is right behind leave
        leave_box_state: false,
    }
}

fn init_boxed_widgets(window: &mut WidgetBuilder, box_conf: &mut BoxConfig) -> BoxedWidgetGrid {
    let mut builder = GrideBoxBuilder::<BoxedWidgetCtx>::new();
    let ws = std::mem::take(&mut box_conf.widgets);

    use config::widgets::wrapbox::BoxedWidget;
    ws.into_iter().for_each(|w| {
        let mut box_temporary_ctx = BoxTemporaryCtx::new(window);

        macro_rules! boxed {
            ($ctx:expr, $w:expr) => {{
                let w = $w;
                $ctx.to_boxed_widget_ctx(w)
            }};
        }

        let boxed_widget_context = match w.widget {
            BoxedWidget::Text(text_config) => {
                boxed!(
                    box_temporary_ctx,
                    widgets::text::init_text(&mut box_temporary_ctx, text_config)
                )
            }
            BoxedWidget::Ring(ring_config) => {
                boxed!(
                    box_temporary_ctx,
                    widgets::ring::init_widget(&mut box_temporary_ctx, ring_config)
                )
            }
            BoxedWidget::Tray(tray_config) => {
                boxed!(
                    box_temporary_ctx,
                    widgets::tray::init_widget(&mut box_temporary_ctx, tray_config)
                )
            }
        };

        builder.add(boxed_widget_context, (w.index[0], w.index[1]));
    });

    builder.build(box_conf.gap, box_conf.align)
}

struct BoxTemporaryCtx<'a, 'b> {
    builder: &'a mut WidgetBuilder<'b>,
    animation_list: AnimationList,
    has_update: Rc<Cell<bool>>,
}
impl<'a, 'b> BoxTemporaryCtx<'a, 'b> {
    fn new(builder: &'a mut WidgetBuilder<'b>) -> Self {
        Self {
            builder,
            animation_list: AnimationList::new(),
            has_update: Rc::new(Cell::new(false)),
        }
    }
    fn new_animation(&mut self, time_cost: u64) -> ToggleAnimationRc {
        self.animation_list.new_transition(time_cost)
    }
    fn redraw_essential(&self) -> impl Fn() + 'static {
        let has_update = Rc::downgrade(&self.has_update);
        move || {
            let Some(has_update) = has_update.upgrade() else {
                return;
            };
            has_update.set(true);
        }
    }
    fn make_redraw_channel<T: 'static>(
        &mut self,
        mut func: impl FnMut(&mut App, T) + 'static,
    ) -> calloop::channel::Sender<T> {
        let update = self.redraw_essential();
        self.builder.make_redraw_channel(move |app, msg| {
            update();
            func(app, msg);
        })
    }
    fn make_redraw_ping_with_func(
        &mut self,
        mut func: impl FnMut(&mut App) + 'static,
    ) -> calloop::ping::Ping {
        let update = self.redraw_essential();
        self.builder.make_redraw_ping_with_func(move |app| {
            update();
            func(app);
        })
    }
    fn make_redraw_ping(&mut self) -> calloop::ping::Ping {
        let update = self.redraw_essential();
        self.builder.make_redraw_ping_with_func(move |_| {
            update();
        })
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_boxed_widget_ctx(self, ctx: impl box_traits::BoxedWidget + 'static) -> BoxedWidgetCtx {
        self.builder.extend_animation_list(&self.animation_list);
        BoxedWidgetCtx::new(ctx, self.animation_list, self.has_update)
    }
}
