//! Spawning external tools.

use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// A GUI-subsystem process makes Windows allocate a console window for every
/// console child it spawns. `main.rs` sets `windows_subsystem = "windows"` for
/// the app itself, but that does not cover children — so yt-dlp and ffmpeg
/// would each flash a black window on every user action.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Build a `Command` for an external tool, without a visible console on Windows.
///
/// Use for every tool the app runs on the user's behalf. Do **not** use for
/// `open`/`explorer`/`xdg-open` — those are meant to show a window.
pub fn command(bin: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(bin);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_keeps_the_program_name() {
        let c = command("yt-dlp");
        assert_eq!(c.get_program(), "yt-dlp");
    }
}
