pub mod backlight;
pub mod pulseaudio;

pub mod common {
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
