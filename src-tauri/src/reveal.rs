use std::{path::Path, process::Command};

use crate::ffmpeg;

pub fn in_explorer(path: &str) -> Result<(), String> {
    let file = ffmpeg::canonical_source(path).map_err(|error| error.to_string())?;
    reveal_file(&file)
}

#[cfg(windows)]
fn reveal_file(file: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(explorer_argument(file))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Unable to open Explorer: {error}"))
}

#[cfg(windows)]
fn explorer_argument(file: &Path) -> String {
    format!(r#"/select,"{}""#, ffmpeg::display_path(file))
}

#[cfg(not(windows))]
fn reveal_file(_file: &Path) -> Result<(), String> {
    Err("Showing a saved file is currently supported only on Windows.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn explorer_argument_selects_the_display_path() {
        let argument = explorer_argument(Path::new(r"\\?\C:\Users\Example User\clip.mp4"));
        assert_eq!(argument, r#"/select,"C:\Users\Example User\clip.mp4""#);
    }
}
