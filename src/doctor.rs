use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use which::which;

use crate::config::LoadedConfig;

#[derive(Debug, Clone)]
pub struct DoctorConfig {
    pub strict: bool,
    pub loaded_config: LoadedConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone)]
struct Check {
    name: String,
    status: Status,
    detail: String,
}

pub fn run(config: DoctorConfig) -> Result<()> {
    let checks = collect_checks(&config.loaded_config);

    let mut passed = 0;
    let mut warned = 0;
    let mut failed = 0;

    if let Some(path) = &config.loaded_config.path {
        println!("config: {}", path.display());
    } else {
        println!("config: not loaded, using built-in defaults");
    }

    for check in &checks {
        match check.status {
            Status::Pass => {
                passed += 1;
                println!("[PASS] {}: {}", check.name, check.detail);
            }
            Status::Warn => {
                warned += 1;
                println!("[WARN] {}: {}", check.name, check.detail);
            }
            Status::Fail => {
                failed += 1;
                println!("[FAIL] {}: {}", check.name, check.detail);
            }
        }
    }

    println!();
    println!("summary: {passed} passed, {warned} warnings, {failed} failed");

    if config.strict && failed > 0 {
        bail!("doctor found {failed} failing checks")
    }

    Ok(())
}

fn collect_checks(config: &LoadedConfig) -> Vec<Check> {
    let mut checks = Vec::new();
    let launch = &config.data.launch;
    let build = &config.data.build;
    let serve = &config.data.serve;

    checks.push(if cfg!(target_os = "linux") {
        pass("host OS", format!("linux ({})", std::env::consts::ARCH))
    } else {
        fail(
            "host OS",
            format!(
                "current host is {} on {}; launch requires Linux",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
        )
    });

    checks.push(match which("ip") {
        Ok(path) => pass("iproute2", path.display().to_string()),
        Err(_) => fail("iproute2", "`ip` command not found".to_owned()),
    });

    let qemu_bin = launch
        .qemu_bin
        .clone()
        .unwrap_or_else(|| "qemu-system-x86_64".to_owned());
    checks.push(command_check("qemu", &qemu_bin));

    let accel = launch.accel.clone().unwrap_or_else(|| "kvm".to_owned());
    if accel == "kvm" {
        checks.extend(kvm_checks());
    } else {
        checks.push(warn(
            "acceleration",
            format!("configured accel is `{accel}`, so /dev/kvm is not required"),
        ));
    }

    checks.push(optional_path_check(
        "busybox",
        build.busybox.as_deref(),
        "not configured; set [build].busybox or pass --busybox",
    ));
    checks.push(optional_path_check(
        "kernel",
        launch.kernel.as_deref(),
        "not configured; set [launch].kernel or pass --kernel",
    ));

    let initramfs_path = launch.initramfs.as_deref().or(build.output.as_deref());
    checks.push(optional_path_check(
        "initramfs",
        initramfs_path,
        "not configured; set [build].output or [launch].initramfs",
    ));

    checks.push(match serve.listen {
        Some(addr) => pass("serve.listen", addr.to_string()),
        None => warn(
            "serve.listen",
            "not configured; default will be 127.0.0.1:8080".to_owned(),
        ),
    });

    checks.push(match &launch.host_api {
        Some(url) if is_likely_http_url(url) => pass("launch.host_api", url.clone()),
        Some(url) => fail("launch.host_api", format!("unsupported URL format: {url}")),
        None => warn(
            "launch.host_api",
            "not configured; default will be http://192.168.100.1:8080/incr".to_owned(),
        ),
    });

    checks.push(match &launch.bridge {
        Some(bridge) => bridge_check(bridge),
        None => warn(
            "launch.bridge",
            "not configured; default will be minivm0".to_owned(),
        ),
    });

    checks
}

fn kvm_checks() -> Vec<Check> {
    let mut checks = Vec::new();
    let path = Path::new("/dev/kvm");

    if !path.exists() {
        checks.push(fail("/dev/kvm", "device does not exist".to_owned()));
        return checks;
    }

    checks.push(pass("/dev/kvm", path.display().to_string()));

    let access = OpenOptions::new().read(true).write(true).open(path);
    checks.push(match access {
        Ok(_) => pass("kvm access", "read/write open succeeded".to_owned()),
        Err(error) => fail("kvm access", error.to_string()),
    });

    checks
}

fn bridge_check(bridge: &str) -> Check {
    let output = std::process::Command::new("ip")
        .args(["link", "show", "dev", bridge])
        .output();

    match output {
        Ok(result) if result.status.success() => pass("launch.bridge", bridge.to_owned()),
        Ok(result) => warn(
            "launch.bridge",
            format!(
                "{bridge} not present yet (ip exited with status {})",
                result.status
            ),
        ),
        Err(error) => warn(
            "launch.bridge",
            format!("unable to inspect bridge: {error}"),
        ),
    }
}

fn command_check(label: &str, cmd: &str) -> Check {
    match which(cmd) {
        Ok(path) => pass(label, path.display().to_string()),
        Err(_) => fail(label, format!("`{cmd}` not found in PATH")),
    }
}

fn optional_path_check(label: &str, path: Option<&Path>, missing_message: &str) -> Check {
    match path {
        Some(path) if path.exists() => pass(label, path.display().to_string()),
        Some(path) => fail(label, format!("missing path {}", path.display())),
        None => warn(label, missing_message.to_owned()),
    }
}

fn is_likely_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn pass(name: impl Into<String>, detail: impl Into<String>) -> Check {
    Check {
        name: name.into(),
        status: Status::Pass,
        detail: detail.into(),
    }
}

fn warn(name: impl Into<String>, detail: impl Into<String>) -> Check {
    Check {
        name: name.into(),
        status: Status::Warn,
        detail: detail.into(),
    }
}

fn fail(name: impl Into<String>, detail: impl Into<String>) -> Check {
    Check {
        name: name.into(),
        status: Status::Fail,
        detail: detail.into(),
    }
}

#[allow(dead_code)]
fn _assert_pathbuf_send_sync()
where
    PathBuf: Send + Sync,
{
}
