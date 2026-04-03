use anyhow::{Result, bail};
use tokio::process::Child;

use crate::launcher::LaunchConfig;
use crate::qemu::QemuBackend;
use crate::qemu::VmLaunchSpec;

pub trait HypervisorBackend {
    fn name(&self) -> &'static str;
    fn spawn_vm(&self, spec: &VmLaunchSpec) -> Result<Child>;
}

pub fn build_backend(config: &LaunchConfig) -> Result<Box<dyn HypervisorBackend>> {
    match config.backend.as_str() {
        "qemu" => Ok(Box::new(QemuBackend::new(
            config.qemu_bin.clone(),
            config.machine.clone(),
            config.accel.clone(),
        ))),
        other => bail!("unsupported backend `{other}`"),
    }
}
