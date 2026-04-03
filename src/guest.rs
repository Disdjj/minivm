use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use tracing::info;

#[derive(Debug, Clone)]
pub struct BuildInitramfsConfig {
    pub busybox: PathBuf,
    pub output: PathBuf,
    pub init_script: Option<PathBuf>,
}

static DEFAULT_INIT_SCRIPT: &str = include_str!("../guest/init");

pub fn default_init_script() -> &'static str {
    DEFAULT_INIT_SCRIPT
}

pub async fn build_initramfs(config: BuildInitramfsConfig) -> Result<()> {
    let busybox = tokio::fs::read(&config.busybox)
        .await
        .with_context(|| format!("failed to read busybox from {}", config.busybox.display()))?;

    let init_script = match &config.init_script {
        Some(path) => tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("failed to read init script from {}", path.display()))?,
        None => DEFAULT_INIT_SCRIPT.to_owned(),
    };

    let initramfs = InitramfsBuilder::new()
        .directory(".")?
        .directory("bin")?
        .directory("dev")?
        .directory("proc")?
        .directory("sys")?
        .directory("tmp")?
        .file("init", 0o755, init_script.as_bytes())?
        .file("bin/busybox", 0o755, &busybox)?
        .build();

    if let Some(parent) = config.output.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    tokio::fs::write(&config.output, initramfs)
        .await
        .with_context(|| format!("failed to write initramfs to {}", config.output.display()))?;

    info!("initramfs written to {}", config.output.display());
    Ok(())
}

/// A tiny "newc" cpio writer.
///
/// The Linux kernel accepts cpio archives directly as initramfs payloads. Using
/// the `newc` format lets us generate everything in pure Rust without depending
/// on host tools like `cpio` or `gzip`, which keeps this repository portable and
/// easy to inspect.
struct InitramfsBuilder {
    archive: Vec<u8>,
    inode: u32,
    mtime: u32,
}

impl InitramfsBuilder {
    fn new() -> Self {
        let mtime = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        Self {
            archive: Vec::new(),
            inode: 1,
            mtime,
        }
    }

    fn directory(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let path = normalize_cpio_path(path.as_ref())?;
        self.push_entry(&path, 0o040755, &[])?;
        Ok(self)
    }

    fn file(mut self, path: impl AsRef<Path>, mode: u32, contents: &[u8]) -> Result<Self> {
        let path = normalize_cpio_path(path.as_ref())?;
        self.push_entry(&path, 0o100000 | mode, contents)?;
        Ok(self)
    }

    fn build(mut self) -> Vec<u8> {
        // Every cpio archive must end with a synthetic trailer entry. The
        // kernel uses it as the logical end-of-archive marker.
        self.push_entry("TRAILER!!!", 0, &[])
            .expect("trailer entry is always valid");
        self.archive
    }

    fn push_entry(&mut self, path: &str, mode: u32, contents: &[u8]) -> Result<()> {
        let namesize = path.len() + 1;
        if namesize > 0xffff_ffffusize {
            bail!("cpio path is too long: {path}");
        }

        let mut header = String::new();
        write!(
            &mut header,
            "070701{ino:08x}{mode:08x}{uid:08x}{gid:08x}{nlink:08x}{mtime:08x}{filesize:08x}{devmajor:08x}{devminor:08x}{rdevmajor:08x}{rdevminor:08x}{namesize:08x}{check:08x}",
            ino = self.inode,
            mode = mode,
            uid = 0,
            gid = 0,
            nlink = 1,
            mtime = self.mtime,
            filesize = contents.len(),
            devmajor = 0,
            devminor = 0,
            rdevmajor = 0,
            rdevminor = 0,
            namesize = namesize,
            check = 0,
        )
        .expect("writing to String cannot fail");

        self.archive.extend_from_slice(header.as_bytes());
        self.archive.extend_from_slice(path.as_bytes());
        self.archive.push(0);
        pad_to_4(&mut self.archive);
        self.archive.extend_from_slice(contents);
        pad_to_4(&mut self.archive);
        self.inode = self.inode.saturating_add(1);
        Ok(())
    }
}

fn normalize_cpio_path(path: &Path) -> Result<String> {
    if path.is_absolute() {
        bail!("cpio entries must be relative paths: {}", path.display());
    }

    let path = path
        .to_str()
        .context("cpio path contains non-utf8 bytes")?
        .trim_start_matches("./")
        .trim_end_matches('/');

    if path.is_empty() {
        return Ok(".".to_owned());
    }

    if path.contains("..") {
        bail!("cpio paths may not contain '..': {path}");
    }

    Ok(path.to_owned())
}

fn pad_to_4(bytes: &mut Vec<u8>) {
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
}

#[allow(dead_code)]
fn ensure_executable(path: &Path) -> Result<()> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("failed to chmod {}", path.display()))?;
    Ok(())
}

#[allow(dead_code)]
fn write_debug_copy(path: &Path, contents: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let mut file =
        fs::File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    file.write_all(contents)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[allow(dead_code)]
fn _assert_pathbuf_send_sync()
where
    PathBuf: Send + Sync,
{
}

#[cfg(test)]
mod tests {
    use super::{InitramfsBuilder, normalize_cpio_path};

    #[test]
    fn normalizes_relative_paths() {
        assert_eq!(
            normalize_cpio_path("bin/busybox".as_ref()).unwrap(),
            "bin/busybox"
        );
        assert_eq!(normalize_cpio_path("./tmp/".as_ref()).unwrap(), "tmp");
        assert!(normalize_cpio_path("../escape".as_ref()).is_err());
    }

    #[test]
    fn emits_trailer_entry() {
        let archive = InitramfsBuilder::new()
            .directory("bin")
            .unwrap()
            .file("init", 0o755, b"#!/bin/busybox sh\n")
            .unwrap()
            .build();

        let haystack = String::from_utf8_lossy(&archive);
        assert!(haystack.contains("TRAILER!!!"));
        assert!(haystack.contains("070701"));
    }
}
