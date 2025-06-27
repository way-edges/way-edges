use cairo::{Context, ImageSurface};

use backend::workspace::WorkspaceData;
use config::widgets::workspace::WorkspaceConfig;
use cosmic_text::Color;
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use util::{
    color::{cairo_set_color, color_mix, color_transition},
    draw::new_surface,
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

    pub default_color: Color,
    pub focus_color: Color,
    pub active_color: Color,
    pub hover_color: Option<Color>,

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
            default_color: w_conf.default_color,
            focus_color: w_conf.focus_color,
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

    let surf = new_surface((conf.length, conf.thickness));
    let ctx = Context::new(&surf).unwrap();

    let y = conf.workspace_transition.borrow().progress_abs();

    let mut item_location = vec![];
    let mut draw_start_pos = 0.;

    let hover_id = hover_data.hover_id;

    let border_width = conf.thickness as f64 / 10.;

    macro_rules! get_target {
        ($d:expr, $c:expr) => {
            if $d.focus > -1 {
                ($d.focus, $c.focus_color)
            } else {
                ($d.active, $c.active_color)
            }
        };
    }

    let (target, target_color) = get_target!(data, conf);
    let (prev_target, _) = get_target!(prev_data, conf);
    let default_color = conf.default_color;

    let sorting: Box<dyn std::iter::Iterator<Item = _>> = if conf.invert_direction {
        Box::new((0..data.workspace_count).rev())
    } else {
        Box::new(0..data.workspace_count)
    };
    sorting.enumerate().for_each(|(index, workspace_index)| {
        // size and color
        let (length, mut color) = if workspace_index == target {
            (
                item_min_length + (item_max_length - item_min_length) * y,
                color_transition(default_color, target_color, y as f32),
            )
        } else if workspace_index == prev_target {
            (
                item_min_length + (item_max_length - item_min_length) * (1. - y),
                color_transition(target_color, default_color, y as f32),
            )
        } else {
            (item_min_length, default_color)
        };

        // mouse hover color
        if let Some(hover_color) = conf.hover_color {
            if workspace_index as isize == hover_id {
                color = color_mix(hover_color, color);
            }
        }

        // draw
        if workspace_index == target {
            cairo_set_color(&ctx, color);
            ctx.rectangle(Z, Z, length, conf.thickness as f64);
            ctx.fill().unwrap();
        } else {
            cairo_set_color(&ctx, target_color);
            ctx.rectangle(Z, Z, length, conf.thickness as f64);
            ctx.fill().unwrap();
            cairo_set_color(&ctx, color);
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
