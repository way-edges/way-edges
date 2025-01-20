use cairo::ImageSurface;
use glib::clone::{Downgrade, Upgrade};
use gtk::{
    gdk::{BUTTON_SECONDARY, RGBA},
    glib,
};
use std::{cell::Cell, rc::Rc};

use super::base::{
    draw::DrawConfig,
    event::{setup_event, ProgressState},
};
use crate::{
    animation::ToggleAnimationRc,
    mouse_state::{MouseEvent, MouseStateData},
    wayland::app::WidgetBuilder,
    widgets::WidgetContext,
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
    vinfo: Rc<Cell<VInfo>>,
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

        let p = self.vinfo.get().vol;
        self.draw_conf.draw(p)
    }

    fn on_mouse_event(&mut self, _: &MouseStateData, event: MouseEvent) -> bool {
        if let Some(p) = self.progress_state.if_change_progress(event.clone()) {
            if !self.only_redraw_on_internal_update {
                let mut vinfo = self.vinfo.get();
                vinfo.vol = p;
                self.vinfo.set(vinfo);
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
                set_mute(&self.device, !self.vinfo.get().is_muted);
            }
            _ => {}
        }

        !self.only_redraw_on_internal_update
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
    let vinfo = Rc::new(Cell::new(VInfo::default()));

    let vinfo_weak = Rc::downgrade(&vinfo);
    let mute_animation_weak = mute_animation.downgrade();
    let redraw_signal = builder.make_redraw_channel(move |_, vinfo: VInfo| {
        let Some(vinfo_old) = vinfo_weak.upgrade() else {
            return;
        };
        let Some(mute_animation) = mute_animation_weak.upgrade() else {
            return;
        };

        if vinfo_old.get().is_muted != vinfo.is_muted {
            mute_animation
                .borrow_mut()
                .set_direction(vinfo.is_muted.into());
        }
        vinfo_old.set(vinfo);
    });
    let backend_id = backend::pulseaudio::register_callback(redraw_signal, device.clone()).unwrap();

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
