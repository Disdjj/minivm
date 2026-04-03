use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use which::which;

use crate::config::{
    BuildFileConfig, FileConfig, LaunchFileConfig, LoadedConfig, ServeFileConfig, write_to_path,
};

#[derive(Debug, Clone)]
pub struct WizardConfig {
    pub output: PathBuf,
    pub force: bool,
    pub loaded_config: LoadedConfig,
}

pub fn run(config: WizardConfig) -> Result<()> {
    let theme = ColorfulTheme::default();

    if config.output.exists()
        && !config.force
        && !Confirm::with_theme(&theme)
            .with_prompt(format!(
                "{} already exists. Overwrite it?",
                config.output.display()
            ))
            .default(false)
            .interact()?
    {
        bail!("aborted without overwriting {}", config.output.display());
    }

    println!("minivm interactive setup");
    println!("Press Enter to accept the suggested values.");
    println!();

    let file_config = build_config(&theme, &config.loaded_config)?;

    if let Some(parent) = config.output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    write_to_path(&config.output, &file_config)?;

    println!();
    println!("wrote config to {}", config.output.display());
    println!("next steps:");
    println!(
        "1. cargo run -- --config {} doctor",
        config.output.display()
    );
    println!(
        "2. cargo run -- --config {} build-initramfs",
        config.output.display()
    );
    println!("3. cargo run -- --config {} serve", config.output.display());
    println!(
        "4. sudo ./target/debug/minivm --config {} launch",
        config.output.display()
    );

    Ok(())
}

fn build_config(theme: &ColorfulTheme, loaded: &LoadedConfig) -> Result<FileConfig> {
    let backend = select_backend(theme, loaded)?;
    let serve_listen = prompt_socket_addr(
        theme,
        "Host API listen address",
        loaded
            .data
            .serve
            .listen
            .unwrap_or_else(|| "192.168.100.1:8080".parse().expect("valid socket default")),
    )?;
    let bridge_name = prompt_string(
        theme,
        "Linux bridge name",
        loaded
            .data
            .launch
            .bridge
            .clone()
            .unwrap_or_else(|| "minivm0".to_owned()),
    )?;
    let subnet_base = prompt_ipv4(
        theme,
        "Guest subnet base",
        loaded
            .data
            .launch
            .subnet_base
            .unwrap_or_else(|| Ipv4Addr::new(192, 168, 100, 0)),
    )?;
    let prefix_len = prompt_u8(
        theme,
        "Guest subnet prefix length",
        loaded.data.launch.prefix_len.unwrap_or(24),
    )?;
    let host_api = prompt_string(
        theme,
        "Host API URL guests will call",
        loaded
            .data
            .launch
            .host_api
            .clone()
            .unwrap_or_else(|| format!("http://{serve_listen}/incr")),
    )?;
    let busybox = prompt_path(
        theme,
        "BusyBox binary path",
        loaded
            .data
            .build
            .busybox
            .clone()
            .or_else(|| which("busybox").ok())
            .unwrap_or_else(|| PathBuf::from("/usr/bin/busybox")),
    )?;
    let build_output = prompt_path(
        theme,
        "Initramfs output path",
        loaded
            .data
            .build
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from("out/initramfs.cpio")),
    )?;
    let init_script = prompt_path(
        theme,
        "Guest init script path",
        loaded
            .data
            .build
            .init_script
            .clone()
            .unwrap_or_else(|| PathBuf::from("guest/init")),
    )?;
    let kernel = prompt_path(
        theme,
        "Guest kernel path",
        loaded
            .data
            .launch
            .kernel
            .clone()
            .or_else(linux_kernel_guess)
            .unwrap_or_else(|| PathBuf::from("/boot/vmlinuz-guest")),
    )?;
    let qemu_bin = prompt_string(
        theme,
        "QEMU binary",
        loaded
            .data
            .launch
            .qemu_bin
            .clone()
            .or_else(|| which("qemu-system-x86_64").ok().map(display_path))
            .unwrap_or_else(|| "qemu-system-x86_64".to_owned()),
    )?;
    let machine = prompt_string(
        theme,
        "QEMU machine type",
        loaded
            .data
            .launch
            .machine
            .clone()
            .unwrap_or_else(|| "microvm".to_owned()),
    )?;
    let accel = prompt_string(
        theme,
        "Acceleration mode",
        loaded
            .data
            .launch
            .accel
            .clone()
            .unwrap_or_else(|| "kvm".to_owned()),
    )?;
    let tap_prefix = prompt_string(
        theme,
        "TAP name prefix",
        loaded
            .data
            .launch
            .tap_prefix
            .clone()
            .unwrap_or_else(|| "mvm".to_owned()),
    )?;
    let count = prompt_usize(
        theme,
        "Default guest count",
        loaded.data.launch.count.unwrap_or(3),
    )?;
    let memory_mib = prompt_u32(
        theme,
        "RAM per guest (MiB)",
        loaded.data.launch.memory_mib.unwrap_or(128),
    )?;
    let workdir = prompt_path(
        theme,
        "Runtime work directory",
        loaded
            .data
            .launch
            .workdir
            .clone()
            .unwrap_or_else(|| PathBuf::from("runtime")),
    )?;
    let skip_tap_setup = Confirm::with_theme(theme)
        .with_prompt("Skip TAP creation in launch?")
        .default(loaded.data.launch.skip_tap_setup.unwrap_or(false))
        .interact()?;
    let keep_taps = Confirm::with_theme(theme)
        .with_prompt("Keep TAP devices after guests exit?")
        .default(loaded.data.launch.keep_taps.unwrap_or(false))
        .interact()?;

    Ok(FileConfig {
        serve: ServeFileConfig {
            listen: Some(serve_listen),
        },
        build: BuildFileConfig {
            busybox: Some(busybox),
            output: Some(build_output.clone()),
            init_script: Some(init_script),
        },
        launch: LaunchFileConfig {
            backend: Some(backend),
            count: Some(count),
            kernel: Some(kernel),
            initramfs: Some(loaded.data.launch.initramfs.clone().unwrap_or(build_output)),
            workdir: Some(workdir),
            host_api: Some(host_api),
            bridge: Some(bridge_name),
            subnet_base: Some(subnet_base),
            prefix_len: Some(prefix_len),
            memory_mib: Some(memory_mib),
            qemu_bin: Some(qemu_bin),
            machine: Some(machine),
            accel: Some(accel),
            tap_prefix: Some(tap_prefix),
            skip_tap_setup: Some(skip_tap_setup),
            keep_taps: Some(keep_taps),
        },
    })
}

fn select_backend(theme: &ColorfulTheme, loaded: &LoadedConfig) -> Result<String> {
    let items = [
        "qemu: working backend for the current MVP",
        "kvm: scaffold only, does not boot guests yet",
    ];

    let default = match loaded.data.launch.backend.as_deref() {
        Some("kvm") => 1,
        _ => 0,
    };

    let selection = Select::with_theme(theme)
        .with_prompt("Select backend")
        .items(items)
        .default(default)
        .interact()?;

    Ok(match selection {
        1 => "kvm".to_owned(),
        _ => "qemu".to_owned(),
    })
}

fn prompt_string(theme: &ColorfulTheme, prompt: &str, default: String) -> Result<String> {
    Input::with_theme(theme)
        .with_prompt(prompt)
        .default(default)
        .interact_text()
        .with_context(|| format!("failed to read prompt `{prompt}`"))
}

fn prompt_path(theme: &ColorfulTheme, prompt: &str, default: PathBuf) -> Result<PathBuf> {
    Ok(PathBuf::from(prompt_string(
        theme,
        prompt,
        display_path(default),
    )?))
}

fn prompt_socket_addr(
    theme: &ColorfulTheme,
    prompt: &str,
    default: SocketAddr,
) -> Result<SocketAddr> {
    let value = prompt_string(theme, prompt, default.to_string())?;
    value
        .parse()
        .with_context(|| format!("invalid socket address for `{prompt}`: {value}"))
}

fn prompt_ipv4(theme: &ColorfulTheme, prompt: &str, default: Ipv4Addr) -> Result<Ipv4Addr> {
    let value = prompt_string(theme, prompt, default.to_string())?;
    value
        .parse()
        .with_context(|| format!("invalid IPv4 address for `{prompt}`: {value}"))
}

fn prompt_u8(theme: &ColorfulTheme, prompt: &str, default: u8) -> Result<u8> {
    let value = prompt_string(theme, prompt, default.to_string())?;
    value
        .parse()
        .with_context(|| format!("invalid integer for `{prompt}`: {value}"))
}

fn prompt_u32(theme: &ColorfulTheme, prompt: &str, default: u32) -> Result<u32> {
    let value = prompt_string(theme, prompt, default.to_string())?;
    value
        .parse()
        .with_context(|| format!("invalid integer for `{prompt}`: {value}"))
}

fn prompt_usize(theme: &ColorfulTheme, prompt: &str, default: usize) -> Result<usize> {
    let value = prompt_string(theme, prompt, default.to_string())?;
    value
        .parse()
        .with_context(|| format!("invalid integer for `{prompt}`: {value}"))
}

fn linux_kernel_guess() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let release = std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()?;
        if !release.status.success() {
            return None;
        }

        let name = String::from_utf8(release.stdout).ok()?.trim().to_owned();
        let candidate = PathBuf::from(format!("/boot/vmlinuz-{name}"));
        candidate.exists().then_some(candidate)
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn display_path(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}
