use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::Command as StdCommand;

use anyhow::{Context, Result, bail};
use tokio::process::Child;
use tracing::{info, warn};

use crate::backend::build_backend;
use crate::net::NetworkPlan;
use crate::qemu::VmLaunchSpec;

#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub count: usize,
    pub kernel: PathBuf,
    pub initramfs: PathBuf,
    pub workdir: PathBuf,
    pub host_api: String,
    pub bridge: String,
    pub subnet_base: Ipv4Addr,
    pub prefix_len: u8,
    pub memory_mib: u32,
    pub backend: String,
    pub qemu_bin: String,
    pub machine: String,
    pub accel: String,
    pub tap_prefix: String,
    pub skip_tap_setup: bool,
    pub keep_taps: bool,
}

pub async fn launch(config: LaunchConfig) -> Result<()> {
    if config.count == 0 {
        bail!("launch count must be at least 1");
    }

    let network = NetworkPlan::new(config.subnet_base, config.prefix_len)?;
    let gateway = network.gateway()?.to_string();
    let backend = build_backend(&config)?;

    tokio::fs::create_dir_all(&config.workdir)
        .await
        .with_context(|| format!("failed to create {}", config.workdir.display()))?;

    let mut children = Vec::with_capacity(config.count);
    let mut created_taps = Vec::new();

    for id in 0..config.count {
        let tap_name = network.tap_name(&config.tap_prefix, id)?;
        let guest_ip_cidr = network.guest_cidr(id)?;
        let mac_address = network.mac_address(id)?;
        let name = format!("vm{id}");
        let serial_log = config.workdir.join(format!("{name}.serial.log"));

        if !config.skip_tap_setup {
            create_tap(&tap_name, &config.bridge)?;
            created_taps.push(tap_name.clone());
        }

        let spec = VmLaunchSpec {
            id,
            name,
            kernel: config.kernel.clone(),
            initramfs: config.initramfs.clone(),
            serial_log,
            host_api: config.host_api.clone(),
            guest_ip_cidr,
            gateway: gateway.clone(),
            tap_name,
            mac_address,
            memory_mib: config.memory_mib,
        };

        children.push(backend.spawn_vm(&spec)?);
    }

    info!(
        "spawned {} guests with backend {}",
        children.len(),
        backend.name()
    );

    let wait_result = wait_for_children(children).await;

    if !config.keep_taps {
        for tap in created_taps {
            if let Err(error) = delete_tap(&tap) {
                warn!("failed to delete tap {tap}: {error:#}");
            }
        }
    }

    wait_result
}

async fn wait_for_children(mut children: Vec<Child>) -> Result<()> {
    for (index, child) in children.iter_mut().enumerate() {
        let status = child
            .wait()
            .await
            .with_context(|| format!("failed while waiting for guest {index}"))?;

        if !status.success() {
            bail!("guest {index} exited with status {status}");
        }

        info!("guest {index} exited cleanly");
    }

    Ok(())
}

fn create_tap(name: &str, bridge: &str) -> Result<()> {
    ensure_linux_host()?;

    // iproute2 gives us a robust and inspectable control path. For the first
    // iteration this is much less error-prone than issuing netlink calls from
    // Rust directly.
    run_ip(["tuntap", "add", "dev", name, "mode", "tap"])?;
    run_ip(["link", "set", name, "master", bridge])?;
    run_ip(["link", "set", name, "up"])?;
    Ok(())
}

fn delete_tap(name: &str) -> Result<()> {
    ensure_linux_host()?;
    run_ip(["link", "delete", name])?;
    Ok(())
}

fn run_ip<const N: usize>(args: [&str; N]) -> Result<()> {
    let status = StdCommand::new("ip")
        .args(args)
        .status()
        .with_context(|| format!("failed to execute ip {}", args.join(" ")))?;

    if !status.success() {
        bail!("ip {} exited with status {status}", args.join(" "));
    }

    Ok(())
}

fn ensure_linux_host() -> Result<()> {
    if cfg!(target_os = "linux") {
        return Ok(());
    }

    bail!(
        "launch requires a Linux host with iproute2; current host is {}",
        std::env::consts::OS
    )
}

#[allow(dead_code)]
fn _assert_pathbuf_send_sync()
where
    PathBuf: Send + Sync,
{
}
