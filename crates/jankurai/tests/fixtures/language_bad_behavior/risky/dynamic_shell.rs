use std::process::Command;

pub fn run(cmd: &str) {
    let _ = Command::new("sh").arg("-c").arg(cmd).status();
}
