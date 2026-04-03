use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::{Child, Command};

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
    pub qemu_bin: String,
    pub machine: String,
    pub accel: String,
}

pub async fn spawn_vm(spec: &VmLaunchSpec) -> Result<Child> {
    let mut command = build_command(spec);
    tracing::info!("spawning {} via {:?}", spec.name, command);

    command.spawn().with_context(|| {
        format!(
            "failed to spawn {} using {}",
            spec.name,
            spec.qemu_bin.as_str()
        )
    })
}

fn build_command(spec: &VmLaunchSpec) -> Command {
    let mut command = Command::new(&spec.qemu_bin);

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
            net_device_model(spec),
            spec.mac_address
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    // The microvm machine removes most legacy PC hardware, which is exactly the
    // spirit we want here. The extra flags keep an ISA serial port available so
    // that `console=ttyS0` still gives us guest logs.
    if spec.machine == "microvm" {
        command.arg("-machine").arg(format!(
            "{},accel={},pit=off,pic=off,rtc=off,isa-serial=on",
            spec.machine, spec.accel
        ));
    } else {
        command
            .arg("-machine")
            .arg(format!("{},accel={}", spec.machine, spec.accel));
    }

    if spec.accel == "kvm" {
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

fn net_device_model(spec: &VmLaunchSpec) -> &'static str {
    if spec.machine == "microvm" {
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
            qemu_bin: "qemu-system-x86_64".into(),
            machine: "microvm".into(),
            accel: "kvm".into(),
        };

        let cmdline = kernel_cmdline(&spec);
        assert!(cmdline.contains("console=ttyS0"));
        assert!(cmdline.contains("minivm.id=7"));
        assert!(cmdline.contains("minivm.host_api=http://192.168.100.1:8080/incr"));
    }

    #[test]
    fn picks_net_device_model_from_machine_type() {
        let mut spec = VmLaunchSpec {
            id: 0,
            name: "vm0".into(),
            kernel: PathBuf::from("bzImage"),
            initramfs: PathBuf::from("initramfs.cpio"),
            serial_log: PathBuf::from("vm0.log"),
            host_api: "http://192.168.100.1:8080/incr".into(),
            guest_ip_cidr: "192.168.100.2/24".into(),
            gateway: "192.168.100.1".into(),
            tap_name: "mvm0".into(),
            mac_address: "02:fc:00:00:00:00".into(),
            memory_mib: 128,
            qemu_bin: "qemu-system-x86_64".into(),
            machine: "microvm".into(),
            accel: "kvm".into(),
        };

        assert_eq!(net_device_model(&spec), "virtio-net-device");
        spec.machine = "q35".into();
        assert_eq!(net_device_model(&spec), "virtio-net-pci");
    }
}
