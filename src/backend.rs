use std::future::Future;
use std::pin::Pin;

use anyhow::{Result, bail};

use crate::kvm::KvmBackend;
use crate::launcher::LaunchConfig;
use crate::qemu::QemuBackend;
use crate::qemu::VmLaunchSpec;

pub trait RunningVm: Send {
    fn label(&self) -> String;
    fn wait(self: Box<Self>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
}

pub trait HypervisorBackend {
    fn name(&self) -> &'static str;
    fn spawn_vm(&self, spec: &VmLaunchSpec) -> Result<Box<dyn RunningVm>>;
}

pub fn build_backend(config: &LaunchConfig) -> Result<Box<dyn HypervisorBackend>> {
    match config.backend.as_str() {
        "qemu" => Ok(Box::new(QemuBackend::new(
            config.qemu_bin.clone(),
            config.machine.clone(),
            config.accel.clone(),
        ))),
        "kvm" => Ok(Box::new(KvmBackend::new())),
        other => bail!("unsupported backend `{other}`"),
    }
}
