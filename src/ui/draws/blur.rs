use gtk::cairo::{Format, ImageSurface};

/// Blur a cairo image surface
pub fn blur_image_surface(surface: &mut ImageSurface, radius: i32) {
    let mut width = surface.width();
    let height = surface.height();
    let temp =
        ImageSurface::create(Format::ARgb32, width, height).expect("Couldn’t create surface");
    let mut kernel = [0u8; 17];
    let size = kernel.len() as i32;
    let half = size / 2;

    match surface.format() {
        Format::A1 => return,

        Format::A8 => width /= 4,
        _ => (),
    }

    let src_stride = surface.stride();
    let dst_stride = temp.stride();
    surface
        .with_data(move |src| {
            temp.with_data(move |dst| {
                let src =
                    unsafe { std::slice::from_raw_parts_mut(src.as_ptr() as *mut u8, src.len()) };
                let dst =
                    unsafe { std::slice::from_raw_parts_mut(dst.as_ptr() as *mut u8, dst.len()) };
                let mut x: u32;
                let mut y: u32;
                let mut z: u32;
                let mut w: u32;
                let mut p: u32;

                let mut a: u32 = 0;
                for i in 0..size {
                    let f = i - half;
                    let f = f as f64;
                    kernel[i as usize] = ((-f * f / 30.0).exp() * 80.0) as u8;
                    a += kernel[i as usize] as u32;
                }

                // Horizontally blur from surface -> temp
                for i in 0..height {
                    let s: &[u32] = unsafe { src[(i * src_stride) as usize..].align_to::<u32>().1 };
                    let d: &mut [u32] =
                        unsafe { dst[(i * dst_stride) as usize..].align_to_mut::<u32>().1 };
                    for j in 0..width {
                        if radius < j && j < width - radius {
                            let j = j as usize;
                            d[j] = s[j];
                            continue;
                        }

                        x = 0;
                        y = 0;
                        z = 0;
                        w = 0;
                        for k in 0..size {
                            if j - half + k < 0 || j - half + k >= width {
                                continue;
                            }

                            p = s[(j - half + k) as usize];
                            let k = k as usize;

                            x += ((p >> 24) & 0xff) * kernel[k] as u32;
                            y += ((p >> 16) & 0xff) * kernel[k] as u32;
                            z += ((p >> 8) & 0xff) * kernel[k] as u32;
                            w += (p & 0xff) * kernel[k] as u32;
                        }
                        d[j as usize] = (x / a) << 24 | (y / a) << 16 | (z / a) << 8 | (w / a);
                    }
                }

                // Then vertically blur from tmp -> surface
                for i in 0..height {
                    let mut s: &mut [u32] =
                        unsafe { dst[(i * dst_stride) as usize..].align_to_mut::<u32>().1 };
                    let d: &mut [u32] =
                        unsafe { src[(i * src_stride) as usize..].align_to_mut::<u32>().1 };
                    for j in 0..width {
                        if radius < i && i < height - radius {
                            let j = j as usize;
                            d[j] = s[j];
                            continue;
                        }

                        x = 0;
                        y = 0;
                        z = 0;
                        w = 0;
                        for k in 0..size {
                            if i - half + k < 0 || i - half + k >= height {
                                continue;
                            }

                            s = unsafe {
                                dst[((i - half + k) * dst_stride) as usize..]
                                    .align_to_mut::<u32>()
                                    .1
                            };
                            p = s[j as usize];
                            let k = k as usize;
                            x += ((p >> 24) & 0xff) * kernel[k] as u32;
                            y += ((p >> 16) & 0xff) * kernel[k] as u32;
                            z += ((p >> 8) & 0xff) * kernel[k] as u32;
                            w += (p & 0xff) * kernel[k] as u32;
                        }
                        d[j as usize] = (x / a) << 24 | (y / a) << 16 | (z / a) << 8 | (w / a);
                    }
                }
            })
            .unwrap();
        })
        .unwrap();

    surface.mark_dirty();
}
