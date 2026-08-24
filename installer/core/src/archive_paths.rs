//! Fixing up archives that store Windows path separators.
//!
//! AeroNav/GNG packages are produced on Windows, and some archivers write the
//! entry names with `\` separators. Extracting such an archive on Windows yields
//! real nested directories; on macOS/Linux `\` is an ordinary filename
//! character, so the whole package lands as a handful of files literally named
//! `LFBB\ICAO\LFBB.txt` — the planner then walks the tree, finds no `LFBB`
//! folder and no sector files, and silently installs nothing.
//!
//! [`normalize_windows_separators`] rewrites those entries into the nested paths
//! they were meant to be. It is a no-op on Windows (where `\` cannot appear in a
//! filename) and on archives that already use `/`.

use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// Split a `\`-separated name into path components, dropping anything that
/// would escape the extraction root (`..`, absolute prefixes, drive letters).
fn safe_nested_path(name: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for segment in name.split('\\') {
        let segment = segment.trim();
        if segment.is_empty() || segment == "." {
            continue;
        }
        // `..`, `C:` and friends: refuse the whole entry rather than guess.
        if segment == ".." || segment.ends_with(':') {
            return None;
        }
        out.push(segment);
    }
    if out.components().count() < 2 {
        return None;
    }
    // Belt and braces: reject anything that is not a plain relative path.
    if !out.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(out)
}

/// Rewrite every file under `root` whose name embeds `\` separators into the
/// equivalent nested path. Returns how many files were moved.
///
/// Directories left empty by the moves are kept — they hold no files, so the
/// planner ignores them.
pub fn normalize_windows_separators(root: &Path) -> std::io::Result<usize> {
    // Collect first: renaming while walking would invalidate the iterator.
    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if !name.contains('\\') {
            continue;
        }
        let Some(nested) = safe_nested_path(&name) else {
            tracing::warn!(entry = %entry.path().display(), "skipping unsafe archive entry name");
            continue;
        };
        let parent = entry.path().parent().unwrap_or(root);
        moves.push((entry.path().to_path_buf(), parent.join(nested)));
    }

    let mut moved = 0usize;
    for (src, dst) in moves {
        if dst.exists() {
            // Two entries claim the same destination — keep the first and leave
            // the duplicate where it is rather than losing data silently.
            tracing::warn!(dst = %dst.display(), "archive entry collides with an existing file");
            continue;
        }
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&src, &dst)?;
        moved += 1;
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_are_left_alone() {
        assert_eq!(safe_nested_path("LFBB.sct"), None);
        assert_eq!(safe_nested_path("no-separators-here"), None);
    }

    #[test]
    fn nested_names_split_into_components() {
        assert_eq!(
            safe_nested_path("LFBB\\ICAO\\LFBB.txt"),
            Some(PathBuf::from("LFBB").join("ICAO").join("LFBB.txt"))
        );
    }

    #[test]
    fn traversal_and_absolute_names_are_refused() {
        assert_eq!(safe_nested_path("..\\..\\evil.txt"), None);
        assert_eq!(safe_nested_path("C:\\Windows\\evil.txt"), None);
        assert_eq!(safe_nested_path("LFBB\\..\\..\\evil.txt"), None);
    }

    // `\` cannot appear in a Windows filename, so the on-disk half of the
    // behaviour is only observable (and only needed) elsewhere.
    #[cfg(not(windows))]
    mod on_disk {
        use super::*;
        use std::fs;
        use tempfile::tempdir;

        #[test]
        fn windows_style_entries_become_real_directories() {
            let tmp = tempdir().unwrap();
            let root = tmp.path();
            fs::write(root.join("LFBB\\ICAO\\LFBB.txt"), b"icao").unwrap();
            fs::write(root.join("LFBB\\Settings\\VoiceChannels.txt"), b"voice").unwrap();
            fs::write(root.join("LFBB-Bordeaux-260301-0003.sct"), b"sector").unwrap();

            assert_eq!(normalize_windows_separators(root).unwrap(), 2);

            assert_eq!(
                fs::read_to_string(root.join("LFBB/ICAO/LFBB.txt")).unwrap(),
                "icao"
            );
            assert_eq!(
                fs::read_to_string(root.join("LFBB/Settings/VoiceChannels.txt")).unwrap(),
                "voice"
            );
            // Untouched, and no second pass needed.
            assert!(root.join("LFBB-Bordeaux-260301-0003.sct").is_file());
            assert_eq!(normalize_windows_separators(root).unwrap(), 0);
        }

        #[test]
        fn nested_below_an_already_extracted_folder() {
            let tmp = tempdir().unwrap();
            let root = tmp.path();
            fs::create_dir_all(root.join("package")).unwrap();
            fs::write(root.join("package").join("LFRR\\NavData\\wpt.txt"), b"x").unwrap();

            assert_eq!(normalize_windows_separators(root).unwrap(), 1);
            assert!(root.join("package/LFRR/NavData/wpt.txt").is_file());
        }

        #[test]
        fn unsafe_entries_are_left_in_place() {
            let tmp = tempdir().unwrap();
            let root = tmp.path();
            fs::write(root.join("..\\escape.txt"), b"x").unwrap();

            assert_eq!(normalize_windows_separators(root).unwrap(), 0);
            assert!(root.join("..\\escape.txt").is_file());
        }
    }
}
