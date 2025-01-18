pub mod draw;
pub mod template;

pub mod shell {
    use std::{process::Command, thread};

    pub fn shell_cmd(value: &str) -> Result<String, String> {
        let mut cmd = Command::new("/bin/sh");
        log::debug!("running command: {value}");
        let res = cmd.arg("-c").arg(value).output();
        let msg = match res {
            Ok(o) => {
                if !o.status.success() {
                    Err(format!(
                        "command exit with code 1: {}",
                        String::from_utf8_lossy(&o.stderr)
                    ))
                } else {
                    Ok(String::from_utf8_lossy(&o.stdout).to_string())
                }
            }
            Err(e) => Err(format!("Error: {e}")),
        };
        if let Err(ref e) = msg {
            log::error!("error running command: {value}\n{e}");
            crate::notify_send("Way-Edges command error", e, true);
        };
        msg
    }
    pub fn shell_cmd_non_block(value: String) {
        thread::spawn(move || shell_cmd(&value));
    }
}

pub fn notify_send(summary: &str, body: &str, is_critical: bool) {
    use notify_rust::Notification;

    let mut n = Notification::new();
    n.summary(summary);
    n.body(body);
    if is_critical {
        n.urgency(notify_rust::Urgency::Critical);
    }
    if let Err(e) = n.show() {
        log::error!("Failed to send notification: \"{summary}\" - \"{body}\"\nError: {e}");
    }
}

pub static Z: f64 = 0.;

#[macro_export]
macro_rules! rc_func {
    ($visibility:vis $name:ident, $t:ty) => {
        $visibility struct $name(std::rc::Rc<$t>);
        impl std::ops::Deref for $name {
            type Target = std::rc::Rc<$t>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        paste! {
            impl gtk::glib::clone::Downgrade for $name {
                type Weak = [<$name Weak>];
                fn downgrade(&self) -> Self::Weak {
                    [<$name Weak>](Rc::downgrade(&self.0))
                }
            }

            $visibility struct [<$name Weak>](std::rc::Weak<$t>);
            impl gtk::glib::clone::Upgrade for [<$name Weak>] {
                type Strong = $name;
                fn upgrade(&self) -> Option<Self::Strong> {
                    self.0.upgrade().map($name)
                }
            }
        }
    };
}

use std::fmt::Debug;
use std::fmt::Display;

pub fn binary_search_within_range<T: Debug + PartialOrd + Copy + Display>(
    l: &[[T; 2]],
    v: T,
) -> isize {
    if l.is_empty() {
        return -1;
    }
    if l.len() == 1 {
        if v >= l[0][0] && v < l[0][1] {
            return 0;
        } else {
            return -1;
        }
    }

    let mut index = l.len() - 1;
    let mut half = l.len();

    fn half_index(index: &mut usize, half: &mut usize, is_left: bool) {
        *half = (*half / 2).max(1);

        if is_left {
            *index -= *half
        } else {
            *index += *half
        }
    }

    half_index(&mut index, &mut half, true);

    loop {
        let current = l[index];

        if v < current[0] {
            if index == 0 || l[index - 1][1] <= v {
                return -1;
            } else {
                half_index(&mut index, &mut half, true);
            }
        } else if v >= current[1] {
            if index == l.len() - 1 || v < l[index + 1][0] {
                return -1;
            } else {
                half_index(&mut index, &mut half, false);
            }
        } else {
            return index as isize;
        }
    }
}

pub fn binary_search_end<T: Debug + PartialOrd + Copy + Display + Default>(l: &[T], v: T) -> isize {
    if l.is_empty() {
        return -1;
    }
    if l.len() == 1 {
        if v >= T::default() && v < l[0] {
            return 0;
        } else {
            return -1;
        }
    }

    let mut index = 0;
    let max_index = l.len() - 1;
    let mut get_half = {
        let mut half = l.len();
        move || {
            half = (half / 2).max(1);
            half
        }
    };

    loop {
        let current = l[index];

        if v < current {
            // if at the first, or there's no smaller to the left
            if index == 0 || v >= l[index - 1] {
                return index as isize;
            }
            index -= get_half();
        } else {
            // if it's the last
            if index == max_index {
                return -1;
            }

            // if smaller than the right
            if v < l[index + 1] {
                return (index + 1) as isize;
            }
            index += get_half();
        }
    }
}

pub struct Or(pub bool);
impl Or {
    pub fn or(&mut self, b: bool) {
        self.0 = self.0 || b
    }
    pub fn res(self) -> bool {
        self.0
    }
}
