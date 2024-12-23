use std::cell::UnsafeCell;
use std::rc::{Rc, Weak};

use cairo::{ImageSurface, RectangleInt, Region};
use gtk::prelude::{DrawingAreaExt, DrawingAreaExtManual, NativeExt, SurfaceExt, WidgetExt};
use gtk::{glib, DrawingArea};
use gtk4_layer_shell::Edge;
use paste::paste;
use util::draw::new_surface;
use util::{rc_func, Z};

use super::context::WindowContext;

pub(super) struct BufferWeak(Weak<UnsafeCell<ImageSurface>>);
impl glib::clone::Upgrade for BufferWeak {
    type Strong = Buffer;
    fn upgrade(&self) -> Option<Self::Strong> {
        self.0.upgrade().map(Buffer)
    }
}

#[derive(Clone)]
pub(super) struct Buffer(Rc<UnsafeCell<ImageSurface>>);
impl Default for Buffer {
    fn default() -> Self {
        Self(Rc::new(UnsafeCell::new(new_surface((0, 0)))))
    }
}
impl glib::clone::Downgrade for Buffer {
    type Weak = BufferWeak;
    fn downgrade(&self) -> Self::Weak {
        BufferWeak(self.0.downgrade())
    }
}
impl Buffer {
    fn update_buffer(&self, new: ImageSurface) {
        unsafe { *self.0.get().as_mut().unwrap() = new }
    }
    fn get_buffer(&self) -> &ImageSurface {
        unsafe { self.0.get().as_ref().unwrap() }
    }
}

fn update_buffer_and_area_size(
    buffer: &Buffer,
    darea: &DrawingArea,
    img: ImageSurface,
    max_size_func: &MaxSizeFunc,
) {
    let new_size = max_size_func((img.width(), img.height()));
    darea.set_content_width(new_size.0);
    darea.set_content_height(new_size.1);
    // darea.set_size_request(new_size.0, new_size.1);
    buffer.update_buffer(img);
}

#[macro_export]
macro_rules! type_impl_redraw_notifier {
    () => (impl Fn(Option<cairo::ImageSurface>) + 'static)
}

type RedrawNotifyFunc = Rc<dyn Fn(Option<ImageSurface>) + 'static>;
impl WindowContext {
    pub fn make_redraw_notifier_dyn(&self) -> RedrawNotifyFunc {
        Rc::new(self.make_redraw_notifier())
    }
    pub fn make_redraw_notifier(&self) -> type_impl_redraw_notifier!() {
        let drawing_area = &self.drawing_area;
        let buffer = &self.image_buffer;
        let max_size_func = &self.max_widget_size_func;
        glib::clone!(
            #[weak]
            drawing_area,
            #[weak]
            buffer,
            #[weak]
            max_size_func,
            move |img| {
                if let Some(img) = img {
                    update_buffer_and_area_size(&buffer, &drawing_area, img, &max_size_func);
                }
                drawing_area.queue_draw();
            }
        )
    }
}

impl WindowContext {
    pub fn set_draw_func(&self, mut cb: Option<impl 'static + FnMut() -> Option<ImageSurface>>) {
        let buffer = &self.image_buffer;
        let window = &self.window;
        let base_draw_func = &self.base_draw_func;
        let max_size_func = &self.max_widget_size_func;
        let start_pos = &self.start_pos;
        let ani = self.window_pop_state.borrow().get_animation();
        let frame_manager = self.frame_manager.clone();
        let func = glib::clone!(
            #[weak]
            buffer,
            #[weak]
            window,
            #[weak]
            base_draw_func,
            #[weak]
            max_size_func,
            #[weak]
            start_pos,
            #[weak]
            ani,
            #[weak]
            frame_manager,
            move |darea: &DrawingArea, ctx: &cairo::Context, w, h| {
                // content
                if let Some(cb) = &mut cb {
                    if let Some(img) = cb() {
                        update_buffer_and_area_size(&buffer, darea, img, &max_size_func);
                    }
                }
                let content = buffer.get_buffer();
                let content_size = (content.width(), content.height());
                let area_size = (w, h);

                // pop animation
                frame_manager.borrow_mut().ensure_animations(darea);
                let progress = ani.borrow_mut().progress();

                // check unfinished animation and redraw frame

                // input area && pop progress
                let pose = base_draw_func(&window, ctx, area_size, content_size, progress);
                start_pos.replace((pose[0], pose[1]));

                ctx.set_source_surface(content, Z, Z).unwrap();
                ctx.paint().unwrap();
            }
        );
        self.drawing_area.set_draw_func(func);
    }
}

rc_func!(pub MaxSizeFunc, dyn Fn((i32, i32)) -> (i32, i32));
pub fn make_max_size_func(edge: Edge, extra: i32) -> MaxSizeFunc {
    macro_rules! what_extra {
        ($size:expr, $extra:expr; H) => {
            $size.0 += $extra
        };
        ($size:expr, $extra:expr; V) => {{
            $size.1 += $extra
        }};
    }
    macro_rules! create_max_size_func {
        ($fn_name:ident, $index:tt) => {
            fn $fn_name(content_size: (i32, i32), extra: i32) -> (i32, i32) {
            let mut new = content_size;
                what_extra!(&mut new, extra; $index);
            new
            }
        };
    }
    create_max_size_func!(horizon, H);
    create_max_size_func!(vertical, V);
    let func = match edge {
        Edge::Left | Edge::Right => horizon,
        Edge::Top | Edge::Bottom => vertical,
        _ => unreachable!(),
    };

    MaxSizeFunc(Rc::new(move |size| func(size, extra)))
}

rc_func!(
    pub BaseDrawFunc,
    dyn Fn(&gtk::ApplicationWindow, &cairo::Context, (i32, i32), (i32, i32), f64) -> [i32; 4]
);
pub fn make_base_draw_func(edge: Edge, position: Edge, extra: i32) -> BaseDrawFunc {
    let visible_y_func = get_visible_y_func(edge);
    let xy_func = get_xy_func(edge, position);
    let inr_func = get_input_region_func(edge, extra);

    BaseDrawFunc(Rc::new(
        move |window, ctx, area_size, content_size, animation_progress| {
            let visible_y = visible_y_func(content_size, animation_progress);
            let [x, y] = xy_func(area_size, content_size, visible_y);
            let pose = [x, y, content_size.0, content_size.1];

            // input region
            if let Some(surf) = window.surface() {
                let inr = inr_func(pose);
                println!("inr: {inr:?}");
                surf.set_input_region(&Region::create_rectangle(&inr));
            }

            // pop in progress
            ctx.translate(x as f64, y as f64);

            pose
        },
    ))
}

fn get_visible_y_func(edge: Edge) -> fn((i32, i32), f64) -> i32 {
    macro_rules! edge_wh {
        ($size:expr, $ts_y:expr; H) => {
            ($size.0 as f64 * $ts_y).ceil() as i32
        };
        ($size:expr, $ts_y:expr; V) => {
            ($size.1 as f64 * $ts_y).ceil() as i32
        };
    }

    macro_rules! create_range_fn {
        ($fn_name:ident, $index:tt) => {
            fn $fn_name(content_size: (i32, i32), ts_y: f64) -> i32 {
                edge_wh!(content_size, ts_y; $index)
            }
        };
    }
    create_range_fn!(content_width, H);
    create_range_fn!(content_height, V);
    match edge {
        Edge::Left | Edge::Right => content_width,
        Edge::Top | Edge::Bottom => content_height,
        _ => unreachable!(),
    }
}

#[allow(clippy::type_complexity)]
fn get_xy_func(edge: Edge, position: Edge) -> fn((i32, i32), (i32, i32), i32) -> [i32; 2] {
    macro_rules! match_x {
        // position left
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_LEFT) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_CENTER) => {
            let $i = (calculate_x_additional($area_size.0, $content_size.0) / 2);
        };
        // position right
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_RIGHT) => {
            let $i = calculate_x_additional($area_size.0, $content_size.0);
        };
        // edge left
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_LEFT) => {
            let $i = (-$content_size.0 + $visible_y);
        };
        // edge right
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_RIGHT) => {
            let a = calculate_x_additional($area_size.0, $content_size.0);
            let $i = ($content_size.0 - $visible_y) + a;
        };
    }
    macro_rules! match_y {
        // position top
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_TOP) => {
            let $i = 0;
        };
        // position middle
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_CENTER) => {
            let $i = (calculate_y_additional($area_size.1, $content_size.1) / 2);
        };
        // position bottom
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; POSITION_BOTTOM) => {
            let $i = calculate_y_additional($area_size.1, $content_size.1);
        };
        // edge top
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_TOP) => {
            let $i = (-$content_size.1 + $visible_y);
        };
        // edge bottom
        ($i:ident, $area_size:expr, $content_size:expr, $visible_y:expr; EDGE_BOTTOM) => {
            let a = calculate_y_additional($area_size.1, $content_size.1);
            let $i = ($content_size.1 - $visible_y) + a;
        };
    }

    macro_rules! create_position_fn {
        ($fn_name:ident, $x_arg:tt, $y_arg:tt, $wh_arg:tt) => {
            #[allow(unused_variables)]
            fn $fn_name(
                area_size: (i32, i32),
                content_size: (i32, i32),
                visible_y: i32,
            )->[i32; 2] {
                match_x!(x, area_size, content_size, visible_y; $x_arg);
                match_y!(y, area_size, content_size, visible_y; $y_arg);
                [x, y]
            }
        };
    }

    create_position_fn!(left_center, EDGE_LEFT, POSITION_CENTER, H);
    create_position_fn!(left_top, EDGE_LEFT, POSITION_TOP, H);
    create_position_fn!(left_bottom, EDGE_LEFT, POSITION_BOTTOM, H);

    create_position_fn!(right_center, EDGE_RIGHT, POSITION_CENTER, H);
    create_position_fn!(right_top, EDGE_RIGHT, POSITION_TOP, H);
    create_position_fn!(right_bottom, EDGE_RIGHT, POSITION_BOTTOM, H);

    create_position_fn!(top_center, POSITION_CENTER, EDGE_TOP, V);
    create_position_fn!(top_left, POSITION_LEFT, EDGE_TOP, V);
    create_position_fn!(top_right, POSITION_RIGHT, EDGE_TOP, V);

    create_position_fn!(bottom_center, POSITION_CENTER, EDGE_BOTTOM, V);
    create_position_fn!(bottom_left, POSITION_LEFT, EDGE_BOTTOM, V);
    create_position_fn!(bottom_right, POSITION_RIGHT, EDGE_BOTTOM, V);

    match (edge, position) {
        // left center
        (Edge::Left, Edge::Left) | (Edge::Left, Edge::Right) => left_center,
        // left top
        (Edge::Left, Edge::Top) => left_top,
        // left bottom
        (Edge::Left, Edge::Bottom) => left_bottom,
        // right center
        (Edge::Right, Edge::Left) | (Edge::Right, Edge::Right) => right_center,
        // right top
        (Edge::Right, Edge::Top) => right_top,
        // right bottom
        (Edge::Right, Edge::Bottom) => right_bottom,
        // top center
        (Edge::Top, Edge::Top) | (Edge::Top, Edge::Bottom) => top_center,
        // top left
        (Edge::Top, Edge::Left) => top_left,
        // top right
        (Edge::Top, Edge::Right) => top_right,
        // bottom center
        (Edge::Bottom, Edge::Top) | (Edge::Bottom, Edge::Bottom) => bottom_center,
        // bottom left
        (Edge::Bottom, Edge::Left) => bottom_left,
        // bottom right
        (Edge::Bottom, Edge::Right) => bottom_right,
        _ => unreachable!(),
    }
}

fn get_input_region_func(edge: Edge, extra: i32) -> impl Fn([i32; 4]) -> RectangleInt {
    macro_rules! match_inr {
        ($l:expr, $extra:expr, TOP) => {
            $l[3] += $extra
        };
        ($l:expr, $extra:expr, BOTTOM) => {
            $l[1] -= $extra;
            $l[3] += $extra
        };
        ($l:expr, $extra:expr, LEFT) => {
            $l[2] += $extra
        };
        ($l:expr, $extra:expr, RIGHT) => {
            $l[0] -= $extra;
            $l[2] += $extra
        };
    }
    macro_rules! create_inr_fn {
        ($fn_name:ident, $b:tt) => {
            fn $fn_name(mut l: [i32; 4], extra: i32) -> RectangleInt {
                match_inr!(&mut l, extra, $b);
                RectangleInt::new(l[0], l[1], l[2], l[3])
            }
        };
    }
    create_inr_fn!(inr_top, TOP);
    create_inr_fn!(inr_bottom, BOTTOM);
    create_inr_fn!(inr_left, LEFT);
    create_inr_fn!(inr_right, RIGHT);

    let get_inr = match edge {
        Edge::Top => inr_top,
        Edge::Bottom => inr_bottom,
        Edge::Left => inr_left,
        Edge::Right => inr_right,
        _ => unreachable!(),
    };

    move |l| get_inr(l, extra)
}

fn calculate_x_additional(area_width: i32, content_width: i32) -> i32 {
    (area_width).max(content_width) - content_width
}
fn calculate_y_additional(area_height: i32, content_height: i32) -> i32 {
    (area_height).max(content_height) - content_height
}
