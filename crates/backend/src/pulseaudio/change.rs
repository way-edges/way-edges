use libpulse_binding::{
    context::Context,
    volume::{ChannelVolumes, Volume},
};

use super::{
    pa::{self, get_default_sink, get_default_source, with_context},
    PulseAudioDevice,
};

fn calculate_volumn(channel_volumns: &mut ChannelVolumes, vol_percentage: f64) {
    let cv_len = channel_volumns.len();
    let v = Volume((vol_percentage * (Volume::NORMAL.0 as f64)) as u32);
    channel_volumns.set(cv_len, v);
}

fn change_sink_vol(ctx: &Context, name: &str, vol_percentage: f64) {
    ctx.introspect().get_sink_info_by_name(name, move |list| {
        if let Some(sink_info) = pa::process_list_result(list) {
            let index = sink_info.index;
            let mut channel_volumns = sink_info.volume;
            calculate_volumn(&mut channel_volumns, vol_percentage);
            with_context(move |ctx| {
                ctx.introspect()
                    .set_sink_volume_by_index(index, &channel_volumns, None)
            });
        };
    });
}

fn change_source_vol(ctx: &Context, name: &str, vol_percentage: f64) {
    ctx.introspect().get_source_info_by_name(name, move |list| {
        if let Some(source_info) = pa::process_list_result(list) {
            let index = source_info.index;
            let mut channel_volumns = source_info.volume;
            calculate_volumn(&mut channel_volumns, vol_percentage);
            with_context(move |ctx| {
                ctx.introspect()
                    .set_source_volume_by_index(index, &channel_volumns, None)
            });
        };
    });
}

// i don't know how to set it with pulseaudio api
pub fn set_vol(os: &PulseAudioDevice, v: f64) {
    pa::with_context(move |ctx| match os {
        PulseAudioDevice::DefaultSink => {
            if let Some(name) = get_default_sink() {
                change_sink_vol(ctx, name, v);
            };
        }
        PulseAudioDevice::DefaultSource => {
            if let Some(name) = get_default_source() {
                change_source_vol(ctx, name, v);
            };
        }
        PulseAudioDevice::NamedSink(name) => {
            change_sink_vol(ctx, name, v);
        }
        PulseAudioDevice::NamedSource(name) => {
            change_source_vol(ctx, name, v);
        }
    })
}

pub fn set_mute(os: &PulseAudioDevice, mute: bool) {
    pa::with_context(move |ctx| {
        let mut ins = ctx.introspect();
        match os {
            PulseAudioDevice::DefaultSink => {
                if let Some(name) = get_default_sink() {
                    ins.set_sink_mute_by_name(name, mute, None);
                };
            }
            PulseAudioDevice::DefaultSource => {
                if let Some(name) = get_default_source() {
                    ins.set_source_mute_by_name(name, mute, None);
                };
            }
            PulseAudioDevice::NamedSink(name) => {
                ins.set_sink_mute_by_name(name, mute, None);
            }
            PulseAudioDevice::NamedSource(name) => {
                ins.set_source_mute_by_name(name, mute, None);
            }
        }
    })
}
