//! A single, small wrapper around the common "Open File" dialog.
//!
//! `GetOpenFileNameW` is used rather than the newer `IFileOpenDialog` because
//! it needs no COM apartment on the calling thread — and the settings window
//! runs on eframe's thread, whose apartment model is not ours to assume.

use windows::core::PCWSTR;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Ask the user for an image. `title` names the dialog, so the flash picker
/// and the wallpaper picker can each say what they are for. Returns `None` if
/// they cancel.
pub fn pick_image(initial: &str, title: &str) -> Option<String> {
    // Null-separated, double-null-terminated pairs, as the API wants them.
    // Built here rather than as a const so the escaping stays readable.
    // Every image extension users actually meet, not only the ones WIC is
    // guaranteed to decode: the renderer degrades gracefully when a picked
    // file turns out not to load, but the dialog should never be the thing
    // that hides a picture from the user.
    //
    // The label is only display text — the pattern is what filters — so it
    // spells the formats out (common ones first, since a narrow dialog box
    // truncates the tail) while the pattern quietly accepts everything,
    // including the .jfif the label skips.
    let mut filter: Vec<u16> = Vec::new();
    for part in [
        "Images (*.jpg, *.jpeg, *.png, *.gif, *.webp, *.bmp, *.tif, *.tiff, *.ico, *.avif, *.heic, *.heif, *.svg)",
        "*.png;*.jpg;*.jpeg;*.jfif;*.bmp;*.gif;*.webp;*.tif;*.tiff;*.ico;*.avif;*.heic;*.heif;*.svg",
        "All files (*.*)",
        "*.*",
    ] {
        filter.extend(part.encode_utf16());
        filter.push(0);
    }
    filter.push(0);

    // The dialog writes the chosen path back into this buffer, so it has to be
    // big enough for a long path and owned for the duration of the call.
    let mut buffer = vec![0u16; 1024];
    let trimmed = initial.trim();
    if !trimmed.is_empty() && trimmed.len() < 1000 {
        let src = wide(trimmed);
        buffer[..src.len()].copy_from_slice(&src);
    }

    let title = wide(title);

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: windows::core::PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        // NOCHANGEDIR matters: without it the dialog moves the whole process's
        // working directory, which would break every relative path we hold.
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    if !ok.as_bool() {
        return None;
    }

    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..end]);
    if path.trim().is_empty() {
        None
    } else {
        Some(path)
    }
}

/// Ask the user to pick a Windows application for a notification source.
///
/// The returned path is stored as chosen. Venu does not canonicalize it,
/// check that it still exists, or watch the process -- it is a label that
/// tells the user which application a source refers to. Delivery is decided
/// by the source name and the allowlist, never by this path.
///
/// `OFN_FILEMUSTEXIST` still applies: the file has to be there at the moment
/// the user picks it, which is the only existence check that ever happens.
pub fn pick_executable(initial: &str, title: &str) -> Option<String> {
    // Same dialog as `pick_image`, different filter. Scripts are offered
    // alongside executables because a source is often a .bat or .cmd wrapper
    // rather than a real binary, and "All files" stays available so a user is
    // never blocked by the filter from naming something unusual.
    let mut filter: Vec<u16> = Vec::new();
    for part in [
        "Applications (*.exe, *.bat, *.cmd, *.com)",
        "*.exe;*.bat;*.cmd;*.com",
        "All files (*.*)",
        "*.*",
    ] {
        filter.extend(part.encode_utf16());
        filter.push(0);
    }
    filter.push(0);

    // The dialog writes the chosen path back into this buffer, so it has to
    // be big enough for a long path and owned for the duration of the call.
    let mut buffer = vec![0u16; 1024];
    let trimmed = initial.trim();
    if !trimmed.is_empty() && trimmed.len() < 1000 {
        let src = wide(trimmed);
        buffer[..src.len()].copy_from_slice(&src);
    }

    let title = wide(title);

    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        nFilterIndex: 1,
        lpstrFile: windows::core::PWSTR(buffer.as_mut_ptr()),
        nMaxFile: buffer.len() as u32,
        lpstrTitle: PCWSTR(title.as_ptr()),
        // NOCHANGEDIR for the same reason as pick_image: this dialog runs on
        // eframe's thread, and moving the process working directory would
        // break every relative path Venu holds.
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    let ok = unsafe { GetOpenFileNameW(&mut ofn) };
    if !ok.as_bool() {
        return None;
    }

    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16_lossy(&buffer[..end]);
    if path.trim().is_empty() {
        None
    } else {
        Some(path)
    }
}
