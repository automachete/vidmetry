use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use uuid::Uuid;

pub const PREVIEW_LIMITS: CacheLimits = CacheLimits {
    max_entries: 8,
    max_bytes: 2 * 1024 * 1024 * 1024,
    max_age: Duration::from_secs(30 * 24 * 60 * 60),
};
pub const TIMELINE_LIMITS: CacheLimits = CacheLimits {
    max_entries: 64,
    max_bytes: 128 * 1024 * 1024,
    max_age: Duration::from_secs(30 * 24 * 60 * 60),
};
const STAGING_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Copy)]
pub struct CacheLimits {
    max_entries: usize,
    max_bytes: u64,
    max_age: Duration,
}

struct CacheEntry {
    path: PathBuf,
    size: u64,
    last_used: SystemTime,
    retained: bool,
}

pub fn reusable_entry(path: &Path) -> io::Result<bool> {
    let valid = path.metadata().map(|item| item.len() > 0).unwrap_or(false);
    if valid {
        touch_or_warn(path);
    } else if path.exists() {
        fs::remove_file(path)?;
        remove_access_marker(path);
    }
    Ok(valid)
}

pub fn staging_path(final_path: &Path) -> io::Result<PathBuf> {
    let parent = final_path
        .parent()
        .ok_or_else(|| io::Error::other("cache entry has no parent"))?;
    let stem = final_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("entry");
    let extension = final_path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::other("cache entry has no extension"))?;
    Ok(parent.join(format!(
        ".{stem}.vidmetry-{}.part.{extension}",
        Uuid::new_v4()
    )))
}

pub fn commit(staged: &Path, final_path: &Path) -> io::Result<()> {
    if staged.metadata().map(|item| item.len()).unwrap_or(0) == 0 {
        remove_file_if_present(staged);
        return Err(io::Error::other("cache staging output is empty"));
    }

    if reusable_entry(final_path)? {
        remove_file_if_present(staged);
        return Ok(());
    }

    match fs::rename(staged, final_path) {
        Ok(()) => {
            touch_or_warn(final_path);
            Ok(())
        }
        Err(_error) if reusable_entry(final_path).unwrap_or(false) => {
            remove_file_if_present(staged);
            Ok(())
        }
        Err(error) => {
            remove_file_if_present(staged);
            Err(error)
        }
    }
}

pub fn prune(root: &Path, retained_path: &Path, limits: CacheLimits) -> io::Result<()> {
    let now = SystemTime::now();
    let mut entries = Vec::new();

    for item in fs::read_dir(root)? {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                log::warn!("unable to inspect a cache directory entry: {error}");
                continue;
            }
        };
        let path = item.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if is_staging_file(file_name) {
            if older_than(&path, now, STAGING_MAX_AGE) {
                remove_file_if_present(&path);
            }
            continue;
        }
        if file_name.ends_with(".access") {
            continue;
        }
        let metadata = match path.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                log::warn!(
                    "unable to read cache metadata for {}: {error}",
                    path.display()
                );
                continue;
            }
        };
        if metadata.len() == 0 {
            remove_file_if_present(&path);
            remove_access_marker(&path);
            continue;
        }
        let last_used = access_marker(&path)
            .metadata()
            .and_then(|item| item.modified())
            .or_else(|_| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push(CacheEntry {
            retained: path == retained_path,
            path,
            size: metadata.len(),
            last_used,
        });
    }

    entries.sort_by_key(|entry| entry.last_used);
    let mut count = entries.len();
    let mut bytes = entries.iter().map(|entry| entry.size).sum::<u64>();
    for entry in entries {
        let expired = now
            .duration_since(entry.last_used)
            .map(|age| age > limits.max_age)
            .unwrap_or(false);
        if entry.retained || (!expired && count <= limits.max_entries && bytes <= limits.max_bytes)
        {
            continue;
        }
        match fs::remove_file(&entry.path) {
            Ok(()) => {
                remove_access_marker(&entry.path);
                count = count.saturating_sub(1);
                bytes = bytes.saturating_sub(entry.size);
            }
            Err(error) => log::warn!(
                "unable to remove expired cache entry {}: {error}",
                entry.path.display()
            ),
        }
    }
    remove_orphan_access_markers(root);
    Ok(())
}

fn touch(path: &Path) -> io::Result<()> {
    fs::write(access_marker(path), [])
}

fn touch_or_warn(path: &Path) {
    if let Err(error) = touch(path) {
        log::warn!(
            "unable to update cache access time for {}: {error}",
            path.display()
        );
    }
}

fn access_marker(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".access");
    PathBuf::from(name)
}

fn remove_access_marker(path: &Path) {
    remove_file_if_present(&access_marker(path));
}

fn remove_orphan_access_markers(root: &Path) {
    let items = match fs::read_dir(root) {
        Ok(items) => items,
        Err(error) => {
            log::warn!(
                "unable to inspect cache access markers in {}: {error}",
                root.display()
            );
            return;
        }
    };
    for item in items {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                log::warn!("unable to inspect a cache access marker: {error}");
                continue;
            }
        };
        let path = item.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(data_name) = name.strip_suffix(".access") else {
            continue;
        };
        if !root.join(data_name).exists() {
            remove_file_if_present(&path);
        }
    }
}

fn is_staging_file(file_name: &str) -> bool {
    file_name.starts_with('.') && file_name.contains(".vidmetry-") && file_name.contains(".part.")
}

fn older_than(path: &Path, now: SystemTime, maximum_age: Duration) -> bool {
    path.metadata()
        .and_then(|item| item.modified())
        .ok()
        .and_then(|modified| now.duration_since(modified).ok())
        .is_some_and(|age| age > maximum_age)
}

fn remove_file_if_present(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => log::warn!("unable to remove cache file {}: {error}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("vidmetry-cache-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create cache fixture");
        root
    }

    #[test]
    fn commits_only_nonempty_staging_files() {
        let root = fixture_root();
        let final_path = root.join("entry.mp4");
        let staged = staging_path(&final_path).expect("create staging path");
        fs::write(&staged, b"video").expect("write staging fixture");

        commit(&staged, &final_path).expect("commit cache entry");

        assert_eq!(fs::read(&final_path).expect("read cache entry"), b"video");
        assert!(!staged.exists());
        assert!(access_marker(&final_path).exists());
        fs::remove_dir_all(root).expect("remove cache fixture");
    }

    #[test]
    fn rejects_and_removes_empty_staging_files() {
        let root = fixture_root();
        let final_path = root.join("entry.jpg");
        let staged = staging_path(&final_path).expect("create staging path");
        fs::write(&staged, []).expect("write empty staging fixture");

        assert!(commit(&staged, &final_path).is_err());
        assert!(!staged.exists());
        assert!(!final_path.exists());
        fs::remove_dir_all(root).expect("remove cache fixture");
    }

    #[test]
    fn prunes_to_limits_without_removing_the_retained_entry() {
        let root = fixture_root();
        let retained = root.join("current.mp4");
        for name in ["old-a.mp4", "old-b.mp4", "current.mp4"] {
            let path = root.join(name);
            fs::write(&path, [1_u8; 8]).expect("write cache fixture");
            touch(&path).expect("touch cache fixture");
        }
        let limits = CacheLimits {
            max_entries: 1,
            max_bytes: 8,
            max_age: Duration::from_secs(3600),
        };

        prune(&root, &retained, limits).expect("prune cache fixture");

        assert!(retained.exists());
        assert_eq!(
            fs::read_dir(&root)
                .expect("read cache fixture")
                .filter_map(Result::ok)
                .filter(|item| !item.file_name().to_string_lossy().ends_with(".access"))
                .count(),
            1
        );
        fs::remove_dir_all(root).expect("remove cache fixture");
    }
}
