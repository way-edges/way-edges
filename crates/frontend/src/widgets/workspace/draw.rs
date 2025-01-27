use cairo::{Context, ImageSurface};

use backend::workspace::WorkspaceData;
use config::widgets::workspace::WorkspaceConfig;
use gdk::{prelude::GdkCairoContextExt, RGBA};
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use util::{
    draw::{color_mix, color_transition, new_surface},
    Z,
};

use crate::animation::ToggleAnimationRc;

use super::event::HoverData;

#[derive(Debug)]
pub struct DrawConf {
    thickness: i32,
    length: i32,
    gap: i32,
    active_increase: f64,

    deactive_color: RGBA,
    active_color: RGBA,
    hover_color: Option<RGBA>,

    workspace_transition: ToggleAnimationRc,

    invert_direction: bool,

    func: fn(&DrawConf, WorkspaceData, WorkspaceData, &mut HoverData) -> ImageSurface,
}
impl DrawConf {
    pub fn new(
        w_conf: &WorkspaceConfig,
        workspace_transition: ToggleAnimationRc,
        edge: Anchor,
    ) -> Self {
        let (thickness, length) = w_conf.size().unwrap();

        let func = match edge {
            Anchor::LEFT | Anchor::RIGHT => draw_vertical,
            Anchor::TOP | Anchor::BOTTOM => draw_horizontal,
            _ => unreachable!(),
        };

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
            func,
        }
    }
    pub fn draw(
        &self,
        data: WorkspaceData,
        prev_data: WorkspaceData,
        hover_data: &mut HoverData,
    ) -> ImageSurface {
        (self.func)(self, data, prev_data, hover_data)
    }
}

fn draw_common_horizontal(
    conf: &DrawConf,
    data: WorkspaceData,
    prev_data: WorkspaceData,
    hover_data: &mut HoverData,
) -> ImageSurface {
    let item_base_length = {
        let up = (conf.length - conf.gap * (data.workspace_count - 1)) as f64;
        up / data.workspace_count as f64
    };
    let item_changable_length = item_base_length * conf.active_increase;

    let item_max_length = item_base_length + item_changable_length;
    let item_min_length =
        item_base_length - item_changable_length / (data.workspace_count - 1).max(1) as f64;

    // let surf = new_surface((self.thickness, self.length));
    let surf = new_surface((conf.length, conf.thickness));
    let ctx = Context::new(&surf).unwrap();

    let y = conf.workspace_transition.borrow().progress_abs();

    let mut item_location = vec![];
    let mut draw_start_pos = 0.;

    let hover_id = hover_data.hover_id;

    let border_width = conf.thickness as f64 / 10.;

    // let a: Vec<(f64, RGBA)> = (1..=data.max_workspace).map(|id| {
    let sorting: Box<dyn std::iter::Iterator<Item = _>> = if conf.invert_direction {
        Box::new((1..=data.workspace_count).rev())
    } else {
        Box::new(1..=data.workspace_count)
    };
    sorting.enumerate().for_each(|(index, id)| {
        // size and color
        let (length, mut color) = if id - 1 == data.focus {
            (
                item_min_length + (item_max_length - item_min_length) * y,
                color_transition(conf.deactive_color, conf.active_color, y as f32),
            )
        } else if id - 1 == prev_data.focus {
            (
                item_min_length + (item_max_length - item_min_length) * (1. - y),
                color_transition(conf.active_color, conf.deactive_color, y as f32),
            )
        } else {
            (item_min_length, conf.deactive_color)
        };

        // mouse hover color
        if let Some(hover_color) = conf.hover_color {
            if id as isize - 1 == hover_id {
                color = color_mix(hover_color, color);
            }
        }

        // draw
        if id - 1 == data.focus {
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
        if (index + 1) as i32 != data.workspace_count {
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

fn draw_horizontal(
    conf: &DrawConf,
    data: WorkspaceData,
    prev_data: WorkspaceData,
    hover_data: &mut HoverData,
) -> ImageSurface {
    draw_common_horizontal(conf, data, prev_data, hover_data)
}

fn draw_vertical(
    conf: &DrawConf,
    data: WorkspaceData,
    prev_data: WorkspaceData,
    hover_data: &mut HoverData,
) -> ImageSurface {
    let common = draw_common_horizontal(conf, data, prev_data, hover_data);
    let surf = new_surface((common.height(), common.width()));
    let ctx = Context::new(&surf).unwrap();
    ctx.rotate(90.0_f64.to_radians());
    ctx.translate(0., -conf.thickness as f64);
    ctx.set_source_surface(&common, Z, Z).unwrap();
    ctx.paint().unwrap();
    surf
}
