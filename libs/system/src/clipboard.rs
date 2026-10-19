use std::io::Read;
use wl_clipboard_rs::copy::{Options as CopyOptions, Source, MimeType as CopyMimeType};
use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType as PasteMimeType, Seat as PasteSeat};

pub fn copy_text(text: &str) -> Result<(), String> {
    // Try wl-copy first if installed, as it handles background serving smoothly
    if let Ok(mut child) = std::process::Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                return Ok(());
            }
        }
    }

    // Native Wayland copy via wl-clipboard-rs
    let opts = CopyOptions::new();
    opts.copy(Source::Bytes(text.as_bytes().to_vec().into_boxed_slice()), CopyMimeType::Text)
        .map_err(|e| e.to_string())
}

pub fn paste_text() -> Result<String, String> {
    // Try wl-paste first if installed
    if let Ok(output) = std::process::Command::new("wl-paste")
        .args(["--no-newline"])
        .output()
    {
        if output.status.success() {
            return String::from_utf8(output.stdout).map_err(|e| e.to_string());
        }
    }

    // Native Wayland paste via wl-clipboard-rs
    match get_contents(ClipboardType::Regular, PasteSeat::Unspecified, PasteMimeType::Text) {
        Ok((mut reader, _mime)) => {
            let mut buf = String::new();
            reader.read_to_string(&mut buf).map_err(|e| e.to_string())?;
            Ok(buf)
        }
        Err(e) => Err(e.to_string()),
    }
}
