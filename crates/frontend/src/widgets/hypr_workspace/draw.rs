use cairo::{Context, ImageSurface};
use gtk::{gdk::RGBA, prelude::GdkCairoContextExt};
use gtk4_layer_shell::Edge;

use backend::hypr_workspace::HyprGlobalData;
use config::widgets::hypr_workspace::HyprWorkspaceConfig;
use util::{
    draw::{color_mix, color_transition, new_surface},
    Z,
};

use crate::animation::ToggleAnimationRc;

use super::event::HoverDataRc;

pub struct DrawCore {
    thickness: i32,
    length: i32,
    gap: i32,
    active_increase: f64,

    deactive_color: RGBA,
    active_color: RGBA,
    hover_color: Option<RGBA>,

    workspace_transition: ToggleAnimationRc,

    invert_direction: bool,
}
impl DrawCore {
    fn new(w_conf: &HyprWorkspaceConfig, workspace_transition: ToggleAnimationRc) -> Self {
        let (thickness, length) = w_conf.size().unwrap();
        Self {
            thickness: thickness.ceil() as i32,
            length: length.ceil() as i32,
            gap: w_conf.gap,
            active_increase: w_conf.active_increase,
            deactive_color: w_conf.deactive_color,
            active_color: w_conf.active_color,
            hover_color: w_conf.hover_color,
            invert_direction: w_conf.invert_direction,
            workspace_transition,
        }
    }
}

fn draw_common_horizontal(
    conf: &DrawCore,
    data: HyprGlobalData,
    hover_data: HoverDataRc,
) -> ImageSurface {
    let item_base_length = {
        let up = (conf.length - conf.gap * (data.max_workspace - 1)) as f64;
        up / data.max_workspace as f64
    };
    let item_changable_length = item_base_length * conf.active_increase;

    let item_max_length = item_base_length + item_changable_length;
    let item_min_length =
        item_base_length - item_changable_length / (data.max_workspace - 1) as f64;

    // let surf = new_surface((self.thickness, self.length));
    let surf = new_surface((conf.length, conf.thickness));
    let ctx = Context::new(&surf).unwrap();

    let y = conf.workspace_transition.borrow().progress_abs();

    let mut item_location = vec![];
    let mut draw_start_pos = 0.;

    let mut hover_data = hover_data.borrow_mut();
    let hover_id = hover_data.hover_id;

    let border_width = conf.thickness as f64 / 10.;

    // let a: Vec<(f64, RGBA)> = (1..=data.max_workspace).map(|id| {
    let sorting: Box<dyn std::iter::Iterator<Item = _>> = if conf.invert_direction {
        Box::new((1..=data.max_workspace).rev())
    } else {
        Box::new(1..=data.max_workspace)
    };
    sorting.enumerate().for_each(|(index, id)| {
        // size and color
        let (length, mut color) = if id == data.current_workspace {
            (
                item_min_length + (item_max_length - item_min_length) * y,
                color_transition(conf.deactive_color, conf.active_color, y as f32),
            )
        } else if id == data.prev_workspace {
            (
                item_min_length + (item_max_length - item_min_length) * (1. - y),
                color_transition(conf.active_color, conf.deactive_color, y as f32),
            )
        } else {
            (item_min_length, conf.deactive_color)
        };

        // mouse hover color
        if let Some(hover_color) = conf.hover_color {
            if id as isize == hover_id {
                color = color_mix(hover_color, color);
            }
        }

        // draw
        if id == data.current_workspace {
            ctx.set_source_color(&color);
            ctx.rectangle(Z, Z, length, conf.thickness as f64);
            ctx.fill().unwrap();
        } else {
            ctx.set_source_color(&conf.active_color);
            ctx.rectangle(Z, Z, length, conf.thickness as f64);
            ctx.fill().unwrap();
            ctx.set_source_color(&color);
            ctx.rectangle(
                border_width,
                border_width,
                length - 2. * border_width,
                conf.thickness as f64 - 2. * border_width,
            );
            ctx.fill().unwrap();
        }
        if (index + 1) as i32 != data.max_workspace {
            ctx.translate(length + conf.gap as f64, Z);
        };

        // record calculated size for mouse locate
        let end = draw_start_pos + length;
        item_location.push([draw_start_pos, draw_start_pos + length]);
        draw_start_pos = end + conf.gap as f64;
    });

    hover_data.update_hover_data(item_location);

    surf
}

fn draw_horizontal(conf: &DrawCore, data: HyprGlobalData, hover_data: HoverDataRc) -> ImageSurface {
    draw_common_horizontal(conf, data, hover_data)
}

fn draw_vertical(conf: &DrawCore, data: HyprGlobalData, hover_data: HoverDataRc) -> ImageSurface {
    let common = draw_common_horizontal(conf, data, hover_data);
    let surf = new_surface((common.height(), common.width()));
    let ctx = Context::new(&surf).unwrap();
    ctx.rotate(90.0_f64.to_radians());
    ctx.translate(0., -conf.thickness as f64);
    ctx.set_source_surface(&common, Z, Z).unwrap();
    ctx.paint().unwrap();
    surf
}

pub fn make_draw_func(
    w_conf: &HyprWorkspaceConfig,
    edge: Edge,
    workspace_transition: ToggleAnimationRc,
) -> impl Fn(HyprGlobalData, HoverDataRc) -> ImageSurface {
    let draw_conf = DrawCore::new(w_conf, workspace_transition);
    let draw_func = match edge {
        Edge::Left | Edge::Right => draw_vertical,
        Edge::Top | Edge::Bottom => draw_horizontal,
        _ => unreachable!(),
    };

    move |data, hover_data| draw_func(&draw_conf, data, hover_data)
}
