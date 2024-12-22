pub mod draw;

pub mod shell {
    use std::{process::Command, thread};

    pub fn shell_cmd(value: String) -> Result<String, String> {
        let mut cmd = Command::new("/bin/sh");
        log::debug!("running command: {value}");
        let res = cmd.arg("-c").arg(&value).output();
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
        thread::spawn(move || shell_cmd(value));
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
