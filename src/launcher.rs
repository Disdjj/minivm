use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::Command as StdCommand;

use anyhow::{Context, Result, bail};
use tokio::process::Child;
use tracing::{info, warn};

use crate::cli::LaunchArgs;
use crate::net::NetworkPlan;
use crate::qemu::{VmLaunchSpec, spawn_vm};

pub async fn launch(args: LaunchArgs) -> Result<()> {
    if args.count == 0 {
        bail!("--count must be at least 1");
    }

    let subnet_base: Ipv4Addr = args
        .subnet_base
        .parse()
        .with_context(|| format!("invalid subnet base {}", args.subnet_base))?;
    let network = NetworkPlan::new(subnet_base, args.prefix_len)?;
    let gateway = network.gateway()?.to_string();

    tokio::fs::create_dir_all(&args.workdir)
        .await
        .with_context(|| format!("failed to create {}", args.workdir.display()))?;

    let mut children = Vec::with_capacity(args.count);
    let mut created_taps = Vec::new();

    for id in 0..args.count {
        let tap_name = network.tap_name(&args.tap_prefix, id)?;
        let guest_ip_cidr = network.guest_cidr(id)?;
        let mac_address = network.mac_address(id)?;
        let name = format!("vm{id}");
        let serial_log = args.workdir.join(format!("{name}.serial.log"));

        if !args.skip_tap_setup {
            create_tap(&tap_name, &args.bridge)?;
            created_taps.push(tap_name.clone());
        }

        let spec = VmLaunchSpec {
            id,
            name,
            kernel: args.kernel.clone(),
            initramfs: args.initramfs.clone(),
            serial_log,
            host_api: args.host_api.clone(),
            guest_ip_cidr,
            gateway: gateway.clone(),
            tap_name,
            mac_address,
            memory_mib: args.memory_mib,
            qemu_bin: args.qemu_bin.clone(),
            machine: args.machine.clone(),
            accel: args.accel.clone(),
        };

        children.push(spawn_vm(&spec).await?);
    }

    info!("spawned {} guests", children.len());

    let wait_result = wait_for_children(children).await;

    if !args.keep_taps {
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
