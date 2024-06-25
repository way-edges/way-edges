use gtk::cairo::Context;
use gtk::cairo::ImageSurface;
use gtk::cairo::LinearGradient;
use gtk::gdk::prelude::*;
use gtk::gdk::RGBA;
use pangocairo::cairo;
use pangocairo::cairo::Format;

fn draw_2(context: &Context, radius: f64, h: f64) {
    let lg_height = h - radius * 2.;

    super::draw_fan_no_close(context, (0., radius), radius, 1., 2.);
    context.move_to(radius, radius);
    context.rel_line_to(0., lg_height);
    super::draw_fan_no_close(context, (0., h - radius), radius, 0., 1.);
    context.rel_line_to(0., -lg_height);
}

pub fn draw_to_surface(
    map_size: (i32, i32),
    item_size: (f64, f64),
    main_color: RGBA,
    extra_trigger_size: f64,
) -> (ImageSurface, ImageSurface, ImageSurface) {
    // size and position
    let f_map_size = (map_size.0 as f64, map_size.1 as f64);
    // let vertical_center = |ctx: &Context| {
    //     ctx.translate(0., (map_size.1 as f64 - item_size.1) / 2.);
    // };

    // color
    let mut start_color = main_color;
    start_color.set_alpha(1.);
    let mut end_color = main_color;
    end_color.set_alpha(0.);

    let path;

    // base_surf
    let base_surf = {
        let base_surf = ImageSurface::create(Format::ARgb32, map_size.0, map_size.1).unwrap();
        let base_ctx = Context::new(&base_surf).unwrap();

        // blur
        {
            let surf = {
                let mut surf = cairo::ImageSurface::create(Format::ARgb32, map_size.0, map_size.1)
                    .expect("Couldn’t create surface");
                let ctx = cairo::Context::new(&surf).unwrap();
                // vertical_center(&ctx);
                // let scale_x = 1. + (1. / item_size.0);
                // ctx.scale(scale_x, 1.);
                draw_2(&ctx, item_size.0, item_size.1);
                ctx.set_source_color(&main_color);
                ctx.fill().unwrap();
                super::blur::blur_image_surface(&mut surf, (extra_trigger_size * 2.) as i32);
                surf
            };
            base_ctx.save().unwrap();
            base_ctx.set_source_surface(&surf, 0., 0.).unwrap();
            base_ctx.rectangle(0., 0., f_map_size.0, f_map_size.1);
            base_ctx.fill().unwrap();
            base_ctx.restore().unwrap();
        };

        // core fill
        {
            base_ctx.save().unwrap();
            // vertical_center(&base_ctx);
            draw_2(&base_ctx, item_size.0, item_size.1);
            path = base_ctx.copy_path().unwrap();
            base_ctx.set_source_color(&main_color);
            base_ctx.fill().unwrap();
            base_ctx.restore().unwrap();
        };

        // border
        {
            base_ctx.save().unwrap();
            // vertical_center(&base_ctx);
            base_ctx.append_path(&path);
            base_ctx.stroke().unwrap();
            base_ctx.restore().unwrap();
        };

        base_surf
    };

    // mask
    let (normal_surf, pressing_surf) = {
        let start_point = (0., f_map_size.1 / 2.);
        let end_point = (item_size.0, f_map_size.1 / 2.);

        let normal_surf = {
            let surf = cairo::ImageSurface::create(Format::ARgb32, map_size.0, map_size.1)
                .expect("Couldn’t create surface");
            let ctx = cairo::Context::new(&surf).unwrap();
            let lg = LinearGradient::new(start_point.0, start_point.1, end_point.0, end_point.1);
            lg.add_color_stop_rgba(0., 0., 0., 0., 0.);
            lg.add_color_stop_rgba(0.4, 0., 0., 0., 0.);
            lg.add_color_stop_rgba(1., 0., 0., 0., 0.7);
            ctx.set_source(&lg).unwrap();
            // vertical_center(&ctx);
            ctx.append_path(&path);
            ctx.fill().unwrap();
            surf
        };

        let pressing_surf = {
            let surf = cairo::ImageSurface::create(Format::ARgb32, map_size.0, map_size.1)
                .expect("Couldn’t create surface");
            let ctx = cairo::Context::new(&surf).unwrap();
            let lg = LinearGradient::new(start_point.0, start_point.1, end_point.0, end_point.1);
            lg.add_color_stop_rgba(0., 0., 0., 0., 0.7);
            lg.add_color_stop_rgba(0.45, 0., 0., 0., 0.2);
            lg.add_color_stop_rgba(0.55, 0., 0., 0., 0.);
            lg.add_color_stop_rgba(1., 0., 0., 0., 0.7);
            ctx.set_source(&lg).unwrap();
            // vertical_center(&ctx);
            ctx.append_path(&path);
            ctx.fill().unwrap();
            surf
        };

        (normal_surf, pressing_surf)
    };

    (base_surf, normal_surf, pressing_surf)
}
