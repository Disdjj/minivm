# project overview

Project goal:

- Build the smallest practical prototype that can boot `N` Linux microVMs on a local Linux host, give each guest network access, and have each guest increment one host-side HTTP counter.

Problem being solved:

- The project is an educational stepping stone toward a Firecracker-like system.
- The immediate goal is not to reproduce Firecracker internals.
- The immediate goal is to prove the full control-plane and data-plane loop with the smallest amount of moving pieces.

Current implemented scope:

- A Rust CLI entrypoint in `src/main.rs` and `src/cli.rs`.
- Optional shared config loading from `minivm.toml` in `src/config.rs`.
- An interactive config-generation wizard in `src/wizard.rs`.
- A host-side HTTP counter API in `src/counter_api.rs`.
- A host environment diagnostic command in `src/doctor.rs`.
- An initramfs builder in `src/guest.rs`.
- A guest workload script in `guest/init`.
- Deterministic guest IP, MAC, and TAP allocation in `src/net.rs`.
- A backend abstraction in `src/backend.rs` and a QEMU implementation in `src/qemu.rs`.
- A first KVM host probe and backend scaffold in `src/kvm.rs`.
- A multi-guest launcher in `src/launcher.rs`.
- Linux bridge helper scripts in `scripts/linux_setup_bridge.sh` and `scripts/linux_cleanup_bridge.sh`.

Current non-goals:

- No custom `/dev/kvm` ioctls.
- No custom vCPU run loop.
- No custom virtio device implementation.
- No block devices.
- No metadata service.
- No snapshot or restore.
- No jailer or seccomp isolation.

Current architecture choice:

- QEMU is used as the user-space VMM.
- KVM is used only when QEMU is invoked with `--accel kvm`.
- This keeps the MVP focused on orchestration, guest boot flow, and networking instead of low-level device emulation.
- Backend selection is now explicit in launch configuration, but only `qemu` is implemented.
- The backend seam has been widened so future self-hosted VMMs are not forced to look like subprocesses.
- Shared defaults are intentionally moved out of the command line and into optional config so repeated experiments are easier to run.
- The preferred way to create that config is now the interactive wizard rather than manual editing.

Expected success condition:

- `cargo run -- serve` exposes `/incr` and `/count`.
- `cargo run -- build-initramfs` creates a bootable initramfs archive containing BusyBox and `/init`.
- `minivm launch` starts `N` guests.
- Each guest configures `eth0`, performs one request to the host API, logs the response, and powers off.
- Host `/count` returns `N`.

Planned evolution path:

1. Keep the control plane and guest workload stable.
2. Keep configuration, diagnostics, and launch UX stable while replacing only the backend implementation.
3. Extend the current `src/kvm.rs` scaffold from host probing into boot-only KVM guest creation.
4. Add serial output to the KVM backend.
5. Replace QEMU-provided virtio-net with a custom device model.
6. Add stronger isolation and operational features only after the minimal self-hosted VMM exists.
