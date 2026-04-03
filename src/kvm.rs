use anyhow::{Context, Result, bail};

use crate::backend::{HypervisorBackend, RunningVm};
use crate::qemu::VmLaunchSpec;

#[cfg(target_os = "linux")]
const KVM_GET_API_VERSION_IOCTL: libc::c_ulong = 0xAE00;
#[cfg(target_os = "linux")]
const KVM_CREATE_VM_IOCTL: libc::c_ulong = 0xAE01;

#[derive(Debug, Clone)]
pub struct KvmHostInfo {
    pub api_version: i32,
    pub vm_creation_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct KvmBackend;

impl KvmBackend {
    pub fn new() -> Self {
        Self
    }
}

impl HypervisorBackend for KvmBackend {
    fn name(&self) -> &'static str {
        "kvm"
    }

    fn spawn_vm(&self, spec: &VmLaunchSpec) -> Result<Box<dyn RunningVm>> {
        let info = probe_host().context("failed to probe /dev/kvm before launch")?;

        if let Some(error) = &info.vm_creation_error {
            bail!("kvm backend probe could not create a VM: {error}");
        }

        bail!(
            concat!(
                "backend `kvm` is scaffolded but guest boot is not implemented yet; ",
                "host probe succeeded with KVM API version {}. ",
                "Next step is loading kernel {} into guest memory, creating a vCPU, ",
                "and wiring a serial device before replacing the QEMU backend."
            ),
            info.api_version,
            spec.kernel.display()
        )
    }
}

pub fn probe_host() -> Result<KvmHostInfo> {
    #[cfg(target_os = "linux")]
    {
        use std::fs::OpenOptions;
        use std::os::fd::AsRawFd;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/kvm")
            .context("open /dev/kvm")?;

        let api_version = unsafe {
            // SAFETY: The file descriptor is valid for the lifetime of `file`,
            // the ioctl takes no pointer arguments, and we check the return code.
            libc::ioctl(file.as_raw_fd(), KVM_GET_API_VERSION_IOCTL)
        };
        if api_version < 0 {
            return Err(std::io::Error::last_os_error()).context("KVM_GET_API_VERSION failed");
        }

        let vm_fd = unsafe {
            // SAFETY: The file descriptor is valid and the third argument is the
            // documented machine type selector for KVM_CREATE_VM. Zero means the
            // default machine type on x86_64, which is sufficient for probing.
            libc::ioctl(file.as_raw_fd(), KVM_CREATE_VM_IOCTL, 0)
        };

        let vm_creation_error = if vm_fd < 0 {
            Some(std::io::Error::last_os_error().to_string())
        } else {
            unsafe {
                // SAFETY: `vm_fd` was returned by a successful ioctl call above.
                libc::close(vm_fd);
            }
            None
        };

        return Ok(KvmHostInfo {
            api_version,
            vm_creation_error,
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        bail!("the kvm backend is only available on Linux hosts")
    }
}
