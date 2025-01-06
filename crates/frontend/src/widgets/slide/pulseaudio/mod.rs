use cairo::ImageSurface;
use gtk::{
    gdk::{BUTTON_SECONDARY, RGBA},
    glib,
};
use std::{cell::UnsafeCell, rc::Rc, time::Duration};

use super::base::{
    draw::DrawConfig,
    event::{setup_event, ProgressState},
};
use crate::{
    animation::ToggleAnimationRc,
    mouse_state::{MouseEvent, MouseStateData},
    window::{WidgetContext, WindowContextBuilder},
};

use backend::pulseaudio::{
    change::{set_mute, set_vol},
    PulseAudioDevice, VInfo,
};
use config::{
    widgets::slide::{base::SlideConfig, preset::PulseAudioConfig},
    Config,
};
use util::draw::color_transition;

pub struct PulseAudioContext {
    backend_id: i32,
    device: PulseAudioDevice,
    vinfo: Rc<UnsafeCell<VInfo>>,
    debounce_ctx: Option<Rc<()>>,

    non_mute_color: RGBA,
    mute_color: RGBA,
    mute_animation: ToggleAnimationRc,
    draw_conf: DrawConfig,

    progress_state: ProgressState,
    only_redraw_on_internal_update: bool,
}
impl WidgetContext for PulseAudioContext {
    fn redraw(&mut self) -> Option<ImageSurface> {
        let mute_y = self.mute_animation.borrow_mut().progress();
        let fg_color = color_transition(self.non_mute_color, self.mute_color, mute_y as f32);
        self.draw_conf.fg_color = fg_color;

        let p = unsafe { self.vinfo.get().as_ref().unwrap().vol };
        Some(self.draw_conf.draw(p))
    }

    fn on_mouse_event(&mut self, _: &MouseStateData, event: MouseEvent) -> bool {
        if let Some(p) = self.progress_state.if_change_progress(event.clone()) {
            unsafe { self.vinfo.get().as_mut().unwrap().vol = p }

            // debounce
            if let Some(last) = self.debounce_ctx.take() {
                drop(last)
            }
            let ctx = Rc::new(());
            let device = self.device.clone();
            glib::timeout_add_local_once(
                Duration::from_millis(1),
                glib::clone!(
                    #[weak]
                    ctx,
                    move || {
                        let _ = ctx;
                        set_vol(&device, p);
                    }
                ),
            );
            self.debounce_ctx = Some(ctx);
            // set_vol(&self.device, p);
        }

        match event {
            MouseEvent::Release(_, BUTTON_SECONDARY) => {
                let vinfo = unsafe { self.vinfo.get().as_mut().unwrap() };
                vinfo.is_muted = !vinfo.is_muted;
                set_mute(&self.device, vinfo.is_muted);
                true
            }
            _ => !self.only_redraw_on_internal_update,
        }
    }
}

fn common(
    window: &mut WindowContextBuilder,
    conf: &Config,
    mut w_conf: SlideConfig,
    preset_conf: PulseAudioConfig,
    device: PulseAudioDevice,
) -> impl WidgetContext {
    // TODO: PUT TIME COST INTO CONFIG?
    let mute_animation = window.new_animation(200);
    let non_mute_color = w_conf.fg_color;
    let mute_color = preset_conf.mute_color;
    let vinfo = Rc::new(UnsafeCell::new(VInfo::default()));

    let redraw_signal = window.make_redraw_notifier();

    let vinfo_weak = Rc::downgrade(&vinfo);
    let progress_cache = 0.;
    let backend_id = backend::pulseaudio::register_callback(
        glib::clone!(
            #[weak]
            mute_animation,
            move |vinfo| {
                let Some(vinfo_old) = vinfo_weak.upgrade() else {
                    return;
                };
                let vinfo_old = unsafe { vinfo_old.get().as_mut().unwrap() };
                if vinfo_old == vinfo {
                    if vinfo.vol != progress_cache {
                        redraw_signal();
                    }
                    return;
                }

                if vinfo_old.is_muted != vinfo.is_muted {
                    mute_animation
                        .borrow_mut()
                        .set_direction(vinfo.is_muted.into());
                }
                *vinfo_old = vinfo.clone();
                redraw_signal()
            }
        ),
        device.clone(),
    )
    .unwrap();

    PulseAudioContext {
        backend_id,
        device,
        vinfo,
        debounce_ctx: None,
        non_mute_color,
        mute_color,
        mute_animation,
        draw_conf: DrawConfig::new(&w_conf, conf.edge),
        progress_state: setup_event(conf, &mut w_conf),
        only_redraw_on_internal_update: w_conf.redraw_only_on_internal_update,
    }
}

pub fn speaker(
    window: &mut WindowContextBuilder,
    config: &Config,
    w_conf: SlideConfig,
    mut preset_conf: PulseAudioConfig,
) -> impl WidgetContext {
    let device = preset_conf
        .device
        .take()
        .map_or(PulseAudioDevice::DefaultSink, |name| {
            PulseAudioDevice::NamedSink(name)
        });

    common(window, config, w_conf, preset_conf, device)
}

pub fn microphone(
    window: &mut WindowContextBuilder,
    config: &Config,
    w_conf: SlideConfig,
    mut preset_conf: PulseAudioConfig,
) -> impl WidgetContext {
    let device = preset_conf
        .device
        .take()
        .map_or(PulseAudioDevice::DefaultSource, |name| {
            PulseAudioDevice::NamedSource(name)
        });

    common(window, config, w_conf, preset_conf, device)
}
