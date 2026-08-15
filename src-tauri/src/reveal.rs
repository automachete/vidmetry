use std::{path::Path, process::Command};

use crate::ffmpeg;

pub fn in_explorer(path: &str) -> Result<(), String> {
    let file = ffmpeg::canonical_source(path).map_err(|error| error.to_string())?;
    reveal_file(&file)
}

#[cfg(windows)]
fn reveal_file(file: &Path) -> Result<(), String> {
    let (verb, target) = explorer_arguments(file);
    Command::new("explorer.exe")
        .arg(verb)
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Unable to open Explorer: {error}"))
}

#[cfg(windows)]
fn explorer_arguments(file: &Path) -> (&'static str, String) {
    // Pass the selector and target separately. Embedding quotes inside one
    // argument makes Windows quote the whole token and Explorer may ignore
    // /select, opening the folder without selecting the file.
    ("/select,", ffmpeg::display_path(file))
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
    fn explorer_arguments_select_the_display_path_without_nested_quotes() {
        let arguments = explorer_arguments(Path::new(r"\\?\C:\Users\Example User\clip.mp4"));
        assert_eq!(
            arguments,
            ("/select,", r"C:\Users\Example User\clip.mp4".into())
        );
    }
}
