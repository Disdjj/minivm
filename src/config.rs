use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_CONFIG_PATH: &str = "minivm.toml";

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: Option<PathBuf>,
    pub data: FileConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileConfig {
    #[serde(default)]
    pub serve: ServeFileConfig,
    #[serde(default, rename = "build")]
    pub build: BuildFileConfig,
    #[serde(default)]
    pub launch: LaunchFileConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ServeFileConfig {
    pub listen: Option<SocketAddr>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BuildFileConfig {
    pub busybox: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub init_script: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LaunchFileConfig {
    pub backend: Option<String>,
    pub count: Option<usize>,
    pub kernel: Option<PathBuf>,
    pub initramfs: Option<PathBuf>,
    pub workdir: Option<PathBuf>,
    pub host_api: Option<String>,
    pub bridge: Option<String>,
    pub subnet_base: Option<Ipv4Addr>,
    pub prefix_len: Option<u8>,
    pub memory_mib: Option<u32>,
    pub qemu_bin: Option<String>,
    pub machine: Option<String>,
    pub accel: Option<String>,
    pub tap_prefix: Option<String>,
    pub skip_tap_setup: Option<bool>,
    pub keep_taps: Option<bool>,
}

pub fn default_config_path() -> &'static Path {
    Path::new(DEFAULT_CONFIG_PATH)
}

pub fn write_to_path(path: &Path, config: &FileConfig) -> Result<()> {
    let serialized =
        toml::to_string_pretty(config).context("failed to serialize config as TOML")?;
    std::fs::write(path, serialized)
        .with_context(|| format!("failed to write config to {}", path.display()))?;
    Ok(())
}

pub fn load(path: Option<&Path>) -> Result<LoadedConfig> {
    if let Some(path) = path {
        return load_from_path(path);
    }

    let default = PathBuf::from(DEFAULT_CONFIG_PATH);
    if default.exists() {
        load_from_path(&default)
    } else {
        Ok(LoadedConfig {
            path: None,
            data: FileConfig::default(),
        })
    }
}

fn load_from_path(path: &Path) -> Result<LoadedConfig> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config from {}", path.display()))?;
    let data: FileConfig = toml::from_str(&raw)
        .with_context(|| format!("failed to parse config from {}", path.display()))?;

    Ok(LoadedConfig {
        path: Some(path.to_path_buf()),
        data,
    })
}
