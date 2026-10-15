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

/// Natural comparison without converting arbitrary-size numeric components to integers.
pub fn natural_cmp(left: &Path, right: &Path) -> Ordering {
    let a = casefold(&left.to_string_lossy());
    let b = casefold(&right.to_string_lossy());
    let mut a = a.as_bytes();
    let mut b = b.as_bytes();
    while !a.is_empty() && !b.is_empty() {
        let numeric = a[0].is_ascii_digit() && b[0].is_ascii_digit();
        if numeric {
            let alen = a.iter().take_while(|c| c.is_ascii_digit()).count();
            let blen = b.iter().take_while(|c| c.is_ascii_digit()).count();
            let anum = &a[..alen];
            let bnum = &b[..blen];
            let anum = &anum[anum.iter().take_while(|&&c| c == b'0').count()..];
            let bnum = &bnum[bnum.iter().take_while(|&&c| c == b'0').count()..];
            let order = anum.len().cmp(&bnum.len()).then_with(|| anum.cmp(bnum));
            if order != Ordering::Equal {
                return order;
            }
            a = &a[alen..];
            b = &b[blen..];
        } else {
            let order = a[0].cmp(&b[0]);
            if order != Ordering::Equal {
                return order;
            }
            a = &a[1..];
            b = &b[1..];
        }
    }
    a.len()
        .cmp(&b.len())
        .then_with(|| left.as_os_str().cmp(right.as_os_str()))
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
