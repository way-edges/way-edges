use std::{
    cell::{Cell, RefCell},
    ops::DerefMut,
    rc::Rc,
    thread,
    time::Duration,
};

use libpulse_binding::{
    self as pulse,
    context::subscribe::InterestMaskSet,
    def::Retval,
    mainloop::standard::IterateResult,
    volume::{ChannelVolumes, Volume},
};

use gtk::glib;
use pulse::{
    context::{Context, FlagSet},
    mainloop::standard::Mainloop,
};

struct PA {
    cbs: Vec<Box<dyn FnMut(SubscribeResult)>>,
    on_error_cbs: Vec<Box<dyn FnMut(String)>>,
}

impl PA {
    fn call(&mut self, res: SubscribeResult) {
        self.cbs.iter_mut().for_each(|f| {
            f(res.clone());
        });
    }
    fn error(self, e: String) {
        log::error!("Pulseaudio error(quit mainloop because of this): {e}");
        self.on_error_cbs.into_iter().for_each(|mut f| f(e.clone()));
    }
}

static mut PA_CONTEXT: Option<PA> = None;
fn init_pa() {
    unsafe {
        PA_CONTEXT = Some(PA {
            cbs: vec![],
            on_error_cbs: vec![],
        })
    }
}
fn on_pa_error(e: String) {
    unsafe {
        PA_CONTEXT.take().unwrap().error(e);
    }
}
fn is_pa_inited() -> bool {
    unsafe { PA_CONTEXT.is_some() }
}
fn call_pa(res: SubscribeResult) {
    unsafe {
        PA_CONTEXT.as_mut().unwrap().call(res);
    }
}
fn add_cb(cb: Box<dyn FnMut(SubscribeResult)>) {
    unsafe { PA_CONTEXT.as_mut().unwrap().cbs.push(cb) }
}
fn add_error_cb(cb: Box<dyn FnMut(String)>) {
    unsafe { PA_CONTEXT.as_mut().unwrap().on_error_cbs.push(cb) }
}

#[derive(Debug, Clone)]
pub enum SubscribeResult {
    Sink(f64),
    Source(f64),
}

pub fn try_init_pulseaudio() -> Result<(), String> {
    let is_inited = is_pa_inited();
    if !is_inited {
        let sr = init_mainloop()?;
        glib::spawn_future_local(async move {
            println!("start");
            loop {
                if let Ok(r) = sr.recv().await {
                    println!("recv: {r:#?}");
                    match r {
                        Ok(r) => {
                            call_pa(r);
                        }
                        Err(e) => {
                            on_pa_error(e);
                            break;
                        }
                    }
                } else {
                    on_pa_error(
                        "pulseaudio mainloops seems closed(communication channel closed)"
                            .to_string(),
                    );
                    break;
                }
            }
        });
    }
    Ok(())
}

pub fn register_callback(
    cb: impl FnMut(SubscribeResult) + 'static,
    error_cb: Option<impl FnMut(String) + 'static>,
) -> Result<(), String> {
    try_init_pulseaudio()?;
    add_cb(Box::new(cb));
    if let Some(cb) = error_cb {
        add_error_cb(Box::new(cb));
    }
    Ok(())
}

fn get_avg_volume(cv: ChannelVolumes) -> f64 {
    cv.avg().0 as f64 / Volume::NORMAL.0 as f64
}

fn iter_loop(ml: &mut Mainloop) -> Result<(), String> {
    match ml.iterate(true) {
        IterateResult::Quit(r) => Err(format!("mainloop quit: with status: {r:?}")),
        IterateResult::Err(e) => Err(format!("mainloop iterate Error: {e}")),
        IterateResult::Success(_) => Ok(()),
    }
}

fn init_mainloop() -> Result<async_channel::Receiver<Result<SubscribeResult, String>>, String> {
    // subscribe
    let (ss, sr) = async_channel::bounded::<Result<SubscribeResult, String>>(1);
    {
        // init
        let (ps, pr) = async_channel::bounded::<Result<(), String>>(1);
        thread::spawn(move || {
            let ss_clone = ss.clone();
            let mainloop = move || -> Result<Rc<RefCell<Mainloop>>, String> {
                fn close_mainloop(m: &std::rc::Weak<RefCell<Mainloop>>) {
                    if let Some(m) = m.upgrade() {
                        unsafe {
                            let a = m.as_ptr().as_mut().unwrap();
                            a.quit(Retval(1));
                        }
                    }
                }
                let mainloop = Mainloop::new().ok_or("Failed to create mainloop")?;
                let mut context =
                    Context::new(&mainloop, "Volume Monitor").ok_or("Failed to create context")?;

                context
                    .connect(None, FlagSet::NOAUTOSPAWN, None)
                    .map_err(|e| format!("Failed to connect context: {e}"))?;

                let context = Rc::new(RefCell::new(context));
                let mainloop = Rc::new(RefCell::new(mainloop));

                let ready = Rc::new(Cell::new(false));
                let ready_clone = ready.clone();
                let context_clone = context.clone();
                let mainloop_clone = Rc::downgrade(&mainloop);
                {
                    let ss = ss_clone.clone();
                    context
                        .borrow_mut()
                        .set_state_callback(Some(Box::new(move || {
                            let state = context_clone.borrow().get_state();
                            match state {
                                pulse::context::State::Unconnected => {
                                    close_mainloop(&mainloop_clone);
                                    ss.force_send(Err("PulseAudio callback error".to_string()))
                                        .unwrap();
                                }
                                pulse::context::State::Ready => {
                                    ready_clone.set(true);
                                }
                                _ => {}
                            }
                        })));
                }

                while !ready.get() {
                    iter_loop(mainloop.borrow_mut().deref_mut())?;
                }

                println!("subscribe pulseaudio sink and source");
                {
                    let mut ctx = context.borrow_mut();
                    {
                        let res = Rc::new(Cell::new(None));
                        let res_clone = res.clone();
                        ctx.subscribe(InterestMaskSet::SINK | InterestMaskSet::SOURCE, move |s| {
                            res_clone.set(Some(s));
                        });
                        while res.get().is_none() {
                            iter_loop(mainloop.borrow_mut().deref_mut())?;
                        }
                        let res = res.get().unwrap();
                        if !res {
                            panic!("fail to subscribe pulseaudio");
                        }
                    };
                    {
                        let context_clone = context.clone();
                        let mainloop_clone = Rc::downgrade(&mainloop);
                        let ss = ss_clone.clone();
                        ctx.set_subscribe_callback(Some(Box::new(
                            move |facility, operation, index| {
                                println!(
                                    "{facility:?} event occurred: {:?}, index: {}",
                                    operation, index
                                );
                                let ss = ss.clone();
                                let ins = context_clone.borrow().introspect();
                                let mc = mainloop_clone.clone();
                                match facility.unwrap() {
                                    pulse::context::subscribe::Facility::Sink => {
                                        ins.get_sink_info_by_index(index, move |ls| {
                                            match ls {
                                                pulse::callbacks::ListResult::Item(res) => {
                                                    let avg = get_avg_volume(res.volume);
                                                    println!("sink info: {avg}");
                                                    if ss
                                                        .force_send(Ok(SubscribeResult::Sink(avg)))
                                                        .is_err()
                                                    {
                                                        close_mainloop(&mc);
                                                    }
                                                }
                                                pulse::callbacks::ListResult::End => {}
                                                pulse::callbacks::ListResult::Error => {
                                                    close_mainloop(&mc);
                                                    ss.force_send(Err(
                                                        "Error getting sink info".to_string()
                                                    ))
                                                    .ok();
                                                }
                                            };
                                        });
                                    }
                                    pulse::context::subscribe::Facility::Source => {
                                        ins.get_source_info_by_index(index, move |ls| {
                                            match ls {
                                                pulse::callbacks::ListResult::Item(res) => {
                                                    let avg = get_avg_volume(res.volume);
                                                    if ss
                                                        .force_send(Ok(SubscribeResult::Source(
                                                            avg,
                                                        )))
                                                        .is_err()
                                                    {
                                                        close_mainloop(&mc);
                                                    }
                                                }
                                                pulse::callbacks::ListResult::End => {}
                                                pulse::callbacks::ListResult::Error => {
                                                    close_mainloop(&mc);
                                                    ss.force_send(Err(
                                                        "Error getting source info".to_string()
                                                    ))
                                                    .ok();
                                                }
                                            };
                                        });
                                    }
                                    _ => {}
                                };
                            },
                        )));
                    }
                };
                Ok(mainloop)
            }();
            let mainloop = match mainloop {
                Ok(m) => {
                    if ps.send_blocking(Ok(())).is_err() {
                        m.borrow_mut().quit(Retval(1));
                        return;
                    };
                    m
                }
                Err(e) => {
                    ps.try_send(Err(e)).ok();
                    return;
                }
            };

            println!("wait");

            if let Err(e) = mainloop.borrow_mut().run() {
                ss.force_send(Err(format!("Error running mainloop: {e:?}")))
                    .ok();
            };
            println!("quit");
        });
        pr.recv_blocking().unwrap()?;
    };
    Ok(sr)
}
