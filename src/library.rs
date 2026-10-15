//! File discovery and search, independent of the desktop toolkit.
use anyhow::{Context, Result, bail};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "oga", "opus", "m4a", "aac", "aiff", "aif", "wma", "ape", "alac",
    "mka", "mp2", "ac3", "wv", "amr", "au",
];

pub fn casefold(text: &str) -> String {
    crate::casefold::fold(text)
}

pub fn matches(path: &Path, query: &str) -> bool {
    casefold(&path.to_string_lossy()).contains(&casefold(query))
}

/// Resolve existing ancestors even when the final file is missing (saved playlists).
pub fn absolute(path: &Path) -> Result<PathBuf> {
    let expanded = if path == Path::new("~") {
        PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?)
    } else if let Ok(rest) = path.strip_prefix("~/") {
        PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(rest)
    } else {
        path.to_owned()
    };
    let full = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()?.join(expanded)
    };
    let mut resolved = PathBuf::new();
    for part in full.components() {
        match part {
            Component::ParentDir => {
                resolved.pop();
            }
            Component::CurDir => {}
            other => {
                resolved.push(other.as_os_str());
                if let Ok(canonical) = fs::canonicalize(&resolved) {
                    resolved = canonical;
                }
            }
        }
    }
    Ok(resolved)
}

/// Unicode 16 decimal digits, matching the reference's Python regex/int pair.
/// Each entry begins a ten-codepoint Nd range; data shares assets/UNICODE-LICENSE.txt.
fn decimal(c: char) -> Option<u8> {
    const ZEROS: &[u32] = &[
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10, 0x104a0, 0x10d30, 0x10d40, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450,
        0x114d0, 0x11650, 0x116c0, 0x116d0, 0x116da, 0x11730, 0x118e0, 0x11950, 0x11bf0, 0x11c50,
        0x11d50, 0x11da0, 0x11f50, 0x16130, 0x16a60, 0x16ac0, 0x16b50, 0x16d70, 0x1ccf0, 0x1d7ce,
        0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e4f0, 0x1e5f1, 0x1e950, 0x1fbf0,
    ];
    let code = c as u32;
    let index = ZEROS.partition_point(|zero| *zero <= code).checked_sub(1)?;
    let digit = code - ZEROS[index];
    (digit < 10).then_some(digit as u8)
}

/// Alternating case-folded text and normalized decimal runs. Retain the final
/// text part so "song" precedes "song0", and compare numeric runs without overflow.
fn natural_parts(text: &str) -> Vec<(String, Vec<u8>)> {
    let mut chars = text.chars().peekable();
    let mut parts = Vec::new();
    loop {
        let mut text = String::new();
        while chars.peek().is_some_and(|c| decimal(*c).is_none()) {
            text.push(chars.next().unwrap());
        }
        let at_end = chars.peek().is_none();
        let mut digits = Vec::new();
        while let Some(digit) = chars.peek().and_then(|c| decimal(*c)) {
            digits.push(digit);
            chars.next();
        }
        let zeros = digits.iter().take_while(|n| **n == 0).count();
        digits.drain(..zeros);
        parts.push((casefold(&text), digits));
        if at_end {
            break;
        }
    }
    parts
}

/// Natural comparison preserving stable ties across scripts, case and leading zeros.
pub fn natural_cmp(left: &Path, right: &Path) -> Ordering {
    let a = natural_parts(&left.to_string_lossy());
    let b = natural_parts(&right.to_string_lossy());
    for ((atext, anum), (btext, bnum)) in a.iter().zip(&b) {
        let order = atext
            .cmp(btext)
            .then_with(|| anum.len().cmp(&bnum.len()))
            .then_with(|| anum.cmp(bnum));
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

pub fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|v| v.to_str())
        .is_some_and(|v| AUDIO_EXTENSIONS.contains(&v.to_ascii_lowercase().as_str()))
}

pub fn directory_entries(folder: &Path) -> Result<Vec<(PathBuf, bool, bool)>> {
    fs::read_dir(folder)
        .with_context(|| format!("Could not read {}", folder.display()))?
        .map(|entry| {
            let entry = entry?;
            let metadata = fs::metadata(entry.path())?;
            Ok((
                entry.path(),
                metadata.is_dir(),
                entry.file_type()?.is_symlink(),
            ))
        })
        .collect()
}

pub fn discover(paths: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for value in paths {
        let path = absolute(value)?;
        let metadata =
            fs::metadata(&path).with_context(|| format!("Cannot access {}", value.display()))?;
        let candidates = if metadata.is_dir() {
            let mut pending = vec![path];
            let mut files = Vec::new();
            while let Some(folder) = pending.pop() {
                for (entry, directory, symlink) in directory_entries(&folder)? {
                    if directory {
                        if recursive && !symlink {
                            pending.push(entry);
                        }
                    } else if fs::metadata(&entry)?.is_file() && is_audio(&entry) {
                        files.push(entry);
                    }
                }
            }
            files.sort_by(|a, b| natural_cmp(a, b));
            files
        } else if metadata.is_file() {
            vec![path]
        } else {
            bail!("Not a regular audio file or directory: {}", value.display())
        };
        for candidate in candidates {
            let candidate = fs::canonicalize(candidate)?;
            if seen.insert(candidate.clone()) {
                result.push(candidate);
            }
        }
    }
    Ok(result)
}
