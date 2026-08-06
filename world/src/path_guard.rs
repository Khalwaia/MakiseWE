use std::path::{Component, Path, PathBuf};

use crate::{Result, WorldError};

pub const PROTECTED_MINA_RUNTIME: &str = "/home/artem/kuni_run";

#[derive(Clone, Debug)]
pub struct PathGuard {
    denied_roots: Vec<PathBuf>,
}

impl Default for PathGuard {
    fn default() -> Self {
        Self::new([PathBuf::from(PROTECTED_MINA_RUNTIME)])
    }
}

impl PathGuard {
    pub fn new(denied_roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            denied_roots: denied_roots.into_iter().collect(),
        }
    }

    pub fn validate(&self, candidate: impl AsRef<Path>) -> Result<PathBuf> {
        let candidate = candidate.as_ref();
        if !candidate.is_absolute() {
            return Err(WorldError::UnsafePath {
                path: candidate.to_path_buf(),
                reason: "runtime and data paths must be absolute".into(),
            });
        }

        let lexical = normalize_absolute(candidate)?;
        self.reject_denied(&lexical, "lexical path enters a protected runtime")?;

        let resolved = resolve_existing_prefix(&lexical)?;
        self.reject_denied(&resolved, "resolved path enters a protected runtime")?;
        #[cfg(target_os = "linux")]
        self.reject_mount_aliases(&resolved)?;
        Ok(resolved)
    }

    fn reject_denied(&self, candidate: &Path, reason: &str) -> Result<()> {
        for denied in &self.denied_roots {
            let denied = normalize_absolute(denied)?;
            if candidate == denied || candidate.starts_with(&denied) {
                return Err(WorldError::UnsafePath {
                    path: candidate.to_path_buf(),
                    reason: reason.into(),
                });
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn reject_mount_aliases(&self, candidate: &Path) -> Result<()> {
        let mountinfo = std::fs::read_to_string("/proc/self/mountinfo")?;
        self.reject_mount_aliases_from(candidate, &mountinfo)
    }

    #[cfg(target_os = "linux")]
    fn reject_mount_aliases_from(&self, candidate: &Path, mountinfo: &str) -> Result<()> {
        for line in mountinfo.lines() {
            let Some((mount_root, mount_point, mount_source)) = parse_mountinfo_paths(line) else {
                continue;
            };
            if candidate != mount_point && !candidate.starts_with(&mount_point) {
                continue;
            }
            for denied in &self.denied_roots {
                let denied = normalize_absolute(denied)?;
                let root_is_denied = mount_root == denied || mount_root.starts_with(&denied);
                let source_is_denied = mount_source.is_absolute()
                    && (mount_source == denied || mount_source.starts_with(&denied));
                if root_is_denied || source_is_denied {
                    return Err(WorldError::UnsafePath {
                        path: candidate.to_path_buf(),
                        reason: "bind mount aliases a protected runtime".into(),
                    });
                }
            }
        }
        Ok(())
    }
}

fn normalize_absolute(path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return Err(WorldError::UnsafePath {
                        path: path.to_path_buf(),
                        reason: "path escapes its filesystem root".into(),
                    });
                }
            }
        }
    }
    Ok(normalized)
}

fn resolve_existing_prefix(path: &Path) -> Result<PathBuf> {
    let mut existing = path;
    let mut suffix = Vec::new();
    while !existing.exists() {
        let name = existing.file_name().ok_or_else(|| WorldError::UnsafePath {
            path: path.to_path_buf(),
            reason: "no existing ancestor can be resolved".into(),
        })?;
        suffix.push(name.to_os_string());
        existing = existing.parent().ok_or_else(|| WorldError::UnsafePath {
            path: path.to_path_buf(),
            reason: "no existing ancestor can be resolved".into(),
        })?;
    }

    let mut resolved = std::fs::canonicalize(existing)?;
    for component in suffix.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

#[cfg(target_os = "linux")]
fn parse_mountinfo_paths(line: &str) -> Option<(PathBuf, PathBuf, PathBuf)> {
    let (mount_fields, filesystem_fields) = line.split_once(" - ")?;
    let fields = mount_fields.split_ascii_whitespace().collect::<Vec<_>>();
    let filesystem = filesystem_fields
        .split_ascii_whitespace()
        .collect::<Vec<_>>();
    if fields.len() < 5 || filesystem.len() < 2 {
        return None;
    }
    Some((
        PathBuf::from(decode_mountinfo_field(fields[3])),
        PathBuf::from(decode_mountinfo_field(fields[4])),
        PathBuf::from(decode_mountinfo_field(filesystem[1])),
    ))
}

#[cfg(target_os = "linux")]
fn decode_mountinfo_field(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_mina_runtime_without_touching_it() {
        let guard = PathGuard::default();
        let error = guard
            .validate("/home/artem/kuni_run/data/diary")
            .expect_err("Mina runtime must always be rejected");
        assert!(matches!(error, WorldError::UnsafePath { .. }));
    }

    #[test]
    fn rejects_parent_traversal_into_denied_root() {
        let guard = PathGuard::default();
        assert!(
            guard
                .validate("/home/artem/makise/../kuni_run/data")
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_into_a_denied_root() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let denied = temp.path().join("mina-runtime");
        let safe = temp.path().join("safe");
        std::fs::create_dir_all(&denied).unwrap();
        std::fs::create_dir_all(&safe).unwrap();
        symlink(&denied, safe.join("runtime-link")).unwrap();

        let guard = PathGuard::new([denied]);
        assert!(guard.validate(safe.join("runtime-link/data.db")).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn rejects_bind_mount_alias_from_mountinfo_without_reading_denied_path() {
        let guard = PathGuard::default();
        let mountinfo = concat!(
            "42 31 8:1 /home/artem/kuni_run/data ",
            "/home/artem/makise_run/import rw,relatime - ext4 /dev/sda rw\n"
        );
        assert!(
            guard
                .reject_mount_aliases_from(
                    Path::new("/home/artem/makise_run/import/world.db"),
                    mountinfo
                )
                .is_err()
        );
    }
}
