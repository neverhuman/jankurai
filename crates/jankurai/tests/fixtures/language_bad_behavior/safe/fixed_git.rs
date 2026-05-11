use std::process::Command;

pub fn status() {
    let _ = Command::new("git").args(["status"]).status();
}
