use cairo::ImageSurface;
use gtk::{
    gdk::{BUTTON_SECONDARY, RGBA},
    glib,
};
use std::{
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
};

use super::base::{
    draw::DrawConfig,
    event::{setup_event, ProgressState},
};
use crate::{
    animation::ToggleAnimationRc,
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::{App, WidgetBuilder},
    window::WidgetContext,
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
    #[allow(dead_code)]
    backend_id: i32,
    device: PulseAudioDevice,
    vinfo: Arc<Mutex<VInfo>>,
    debounce_ctx: Option<Rc<()>>,

    non_mute_color: RGBA,
    mute_color: RGBA,
    mute_animation: ToggleAnimationRc,
    draw_conf: DrawConfig,

    progress_state: ProgressState,
    only_redraw_on_internal_update: bool,
}
impl WidgetContext for PulseAudioContext {
    fn redraw(&mut self) -> ImageSurface {
        let mute_y = self.mute_animation.borrow_mut().progress();
        let fg_color = color_transition(self.non_mute_color, self.mute_color, mute_y as f32);
        self.draw_conf.fg_color = fg_color;

        let p = self.vinfo.lock().unwrap().vol;
        self.draw_conf.draw(p)
    }

    fn on_mouse_event(&mut self, _: &MouseStateData, event: MouseEvent) -> bool {
        let mut redraw = false;

        if let Some(p) = self.progress_state.if_change_progress(event.clone()) {
            if !self.only_redraw_on_internal_update {
                let mut vinfo = self.vinfo.lock().unwrap();
                if vinfo.vol != p {
                    vinfo.vol = p;
                    redraw = true
                }
            }
            let device = self.device.clone();
            set_vol(&device, p);

            // debounce
            // if let Some(last) = self.debounce_ctx.take() {
            //     drop(last)
            // }
            // let ctx = Rc::new(());
            // let device = self.device.clone();
            // glib::timeout_add_local_once(
            //     Duration::from_millis(1),
            //     glib::clone!(
            //         #[weak]
            //         ctx,
            //         move || {
            //             let _ = ctx;
            //             set_vol(&device, p);
            //         }
            //     ),
            // );
            // self.debounce_ctx = Some(ctx);
        }

        match event {
            MouseEvent::Release(_, BUTTON_SECONDARY) => {
                let mut vinfo = self.vinfo.lock().unwrap();
                vinfo.is_muted = !vinfo.is_muted;
                set_mute(&self.device, vinfo.is_muted);
                // self.mute_animation
                //     .borrow_mut()
                //     .set_direction(vinfo.is_muted.into());
                // true
            }
            _ => {}
        }

        redraw
    }
}

fn common(
    builder: &mut WidgetBuilder,
    conf: &Config,
    mut w_conf: SlideConfig,
    preset_conf: PulseAudioConfig,
    device: PulseAudioDevice,
) -> impl WidgetContext {
    // TODO: PUT TIME COST INTO CONFIG?
    let mute_animation = builder.new_animation(200);
    let non_mute_color = w_conf.fg_color;
    let mute_color = preset_conf.mute_color;
    let vinfo = Arc::new(Mutex::new(VInfo::default()));

    let redraw_signal = builder.make_redraw_notifier(None::<fn(&mut App)>);

    let vinfo_weak = Arc::downgrade(&vinfo);
    let backend_id = backend::pulseaudio::register_callback(
        glib::clone!(
            #[weak]
            mute_animation,
            move |vinfo| {
                let Some(vinfo_old) = vinfo_weak.upgrade() else {
                    return;
                };
                let mut vinfo_old = vinfo_old.lock().unwrap();
                if vinfo_old.deref() == vinfo {
                    return;
                }

                if vinfo_old.is_muted != vinfo.is_muted {
                    mute_animation
                        .borrow_mut()
                        .set_direction(vinfo.is_muted.into());
                }
                *vinfo_old = vinfo.clone();
                redraw_signal.ping();
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
    builder: &mut WidgetBuilder,
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

    common(builder, config, w_conf, preset_conf, device)
}

pub fn microphone(
    builder: &mut WidgetBuilder,
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

    common(builder, config, w_conf, preset_conf, device)
}
