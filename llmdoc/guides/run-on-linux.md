# run on linux

Purpose:

- Build the current MVP.
- Start the host-side counter service.
- Launch `N` guests.
- Verify that the host count matches `N`.

Prerequisites:

- Linux x86_64 host.
- `/dev/kvm` present if acceleration mode is `kvm`.
- `qemu-system-x86_64` installed.
- `ip` from `iproute2` installed.
- A BusyBox binary available on the host.
- A Linux kernel image suitable for guest boot and containing virtio-net support.

Recommended sequence:

1. Prefer `minivm init` to generate `minivm.toml` interactively. If needed, use `minivm.toml.example` as a static template instead.
2. Build the project with `cargo build`.
3. Run `minivm doctor`.
4. Create the bridge with `scripts/linux_setup_bridge.sh`.
5. Generate the initramfs with `minivm build-initramfs`.
6. Start the host API with `minivm serve`.
7. Launch guests with `minivm launch`.
8. Read `/count` from the host API.
9. Inspect guest serial logs in the launch work directory if anything fails.
10. Remove the bridge with `scripts/linux_cleanup_bridge.sh` when finished.

Typical values:

- Bridge name: `minivm0`
- Bridge address: `192.168.100.1/24`
- Host API URL: `http://192.168.100.1:8080/incr`
- Work directory: `runtime`

Verification checklist:

- `minivm init` produces a config file with the expected local paths and network values.
- `minivm doctor` reports no hard failures for the chosen backend and acceleration mode.
- `GET /healthz` returns `ok`.
- `GET /count` returns `0` before launch.
- Launch exits successfully.
- One serial log file exists per guest under the work directory.
- `GET /count` returns the launched guest count after completion.

Common failure cases:

- `sudo` cannot find `cargo`
  Build with the normal user first and run the compiled binary with `sudo`.

- QEMU cannot use KVM
  Confirm `/dev/kvm` exists and that the current user or root can access it.

- Guest has no `eth0`
  Use a guest kernel with virtio-net enabled.

- Guest cannot reach the host API
  Confirm the bridge exists, the host API is bound to the bridge address, and the guest subnet matches the bridge subnet.

- `launch --backend kvm` fails after probing
  This is currently expected. The `kvm` backend is only a scaffold and does not boot guests yet.
