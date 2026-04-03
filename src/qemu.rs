use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::{Child, Command};

use crate::backend::{HypervisorBackend, RunningVm};

/// Everything QEMU needs to know to boot one guest.
///
/// The launcher computes these values once per VM and hands them to this
/// module. That separation is deliberate: later we can swap this backend for a
/// custom KVM implementation without rewriting the orchestration layer.
#[derive(Debug, Clone)]
pub struct VmLaunchSpec {
    pub id: usize,
    pub name: String,
    pub kernel: PathBuf,
    pub initramfs: PathBuf,
    pub serial_log: PathBuf,
    pub host_api: String,
    pub guest_ip_cidr: String,
    pub gateway: String,
    pub tap_name: String,
    pub mac_address: String,
    pub memory_mib: u32,
}

#[derive(Debug, Clone)]
pub struct QemuBackend {
    qemu_bin: String,
    machine: String,
    accel: String,
}

impl QemuBackend {
    pub fn new(qemu_bin: String, machine: String, accel: String) -> Self {
        Self {
            qemu_bin,
            machine,
            accel,
        }
    }
}

impl HypervisorBackend for QemuBackend {
    fn name(&self) -> &'static str {
        "qemu"
    }

    fn spawn_vm(&self, spec: &VmLaunchSpec) -> Result<Box<dyn RunningVm>> {
        let mut command = build_command(self, spec);
        tracing::info!("spawning {} via {:?}", spec.name, command);

        let child = command.spawn().with_context(|| {
            format!(
                "failed to spawn {} using {}",
                spec.name,
                self.qemu_bin.as_str()
            )
        })?;

        Ok(Box::new(QemuRunningVm {
            name: spec.name.clone(),
            child,
        }))
    }
}

struct QemuRunningVm {
    name: String,
    child: Child,
}

impl RunningVm for QemuRunningVm {
    fn label(&self) -> String {
        self.name.clone()
    }

    fn wait(
        self: Box<Self>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> {
        Box::pin(async move {
            let mut running = *self;
            let status = running
                .child
                .wait()
                .await
                .with_context(|| format!("failed while waiting for {}", running.name))?;

            if !status.success() {
                anyhow::bail!("{} exited with status {status}", running.name);
            }

            Ok(())
        })
    }
}

fn build_command(backend: &QemuBackend, spec: &VmLaunchSpec) -> Command {
    let mut command = Command::new(&backend.qemu_bin);

    command
        .arg("-name")
        .arg(&spec.name)
        .arg("-kernel")
        .arg(&spec.kernel)
        .arg("-initrd")
        .arg(&spec.initramfs)
        .arg("-m")
        .arg(spec.memory_mib.to_string())
        .arg("-smp")
        .arg("1")
        .arg("-append")
        .arg(kernel_cmdline(spec))
        .arg("-nodefaults")
        .arg("-no-user-config")
        .arg("-no-reboot")
        .arg("-display")
        .arg("none")
        .arg("-monitor")
        .arg("none")
        .arg("-serial")
        .arg(format!("file:{}", spec.serial_log.display()))
        .arg("-netdev")
        .arg(format!(
            "tap,id=net0,ifname={},script=no,downscript=no",
            spec.tap_name
        ))
        .arg("-device")
        .arg(format!(
            "{},netdev=net0,mac={}",
            net_device_model(&backend.machine),
            spec.mac_address
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    // The microvm machine removes most legacy PC hardware, which is exactly the
    // spirit we want here. The extra flags keep an ISA serial port available so
    // that `console=ttyS0` still gives us guest logs.
    if backend.machine == "microvm" {
        command.arg("-machine").arg(format!(
            "{},accel={},pit=off,pic=off,rtc=off,isa-serial=on",
            backend.machine, backend.accel
        ));
    } else {
        command
            .arg("-machine")
            .arg(format!("{},accel={}", backend.machine, backend.accel));
    }

    if backend.accel == "kvm" {
        command.arg("-cpu").arg("host");
    }

    command
}

fn kernel_cmdline(spec: &VmLaunchSpec) -> String {
    format!(
        "console=ttyS0 rdinit=/init reboot=t panic=1 minivm.id={} minivm.guest_ip={} minivm.gateway={} minivm.host_api={}",
        spec.id, spec.guest_ip_cidr, spec.gateway, spec.host_api
    )
}

fn net_device_model(machine: &str) -> &'static str {
    if machine == "microvm" {
        "virtio-net-device"
    } else {
        "virtio-net-pci"
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{VmLaunchSpec, kernel_cmdline, net_device_model};

    #[test]
    fn renders_kernel_cmdline() {
        let spec = VmLaunchSpec {
            id: 7,
            name: "vm7".into(),
            kernel: PathBuf::from("bzImage"),
            initramfs: PathBuf::from("initramfs.cpio"),
            serial_log: PathBuf::from("vm7.log"),
            host_api: "http://192.168.100.1:8080/incr".into(),
            guest_ip_cidr: "192.168.100.8/24".into(),
            gateway: "192.168.100.1".into(),
            tap_name: "mvm7".into(),
            mac_address: "02:fc:00:00:00:07".into(),
            memory_mib: 128,
        };

        let cmdline = kernel_cmdline(&spec);
        assert!(cmdline.contains("console=ttyS0"));
        assert!(cmdline.contains("minivm.id=7"));
        assert!(cmdline.contains("minivm.host_api=http://192.168.100.1:8080/incr"));
    }

    #[test]
    fn picks_net_device_model_from_machine_type() {
        assert_eq!(net_device_model("microvm"), "virtio-net-device");
        assert_eq!(net_device_model("q35"), "virtio-net-pci");
    }
}
