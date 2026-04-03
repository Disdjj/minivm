# runtime flow

This document describes the current end-to-end execution path.

Host startup sequence:

1. `minivm init` in `src/wizard.rs` can interactively generate `minivm.toml`.
2. `src/config.rs` optionally loads `minivm.toml` and exposes shared defaults to every CLI subcommand.
3. `minivm doctor` in `src/doctor.rs` can validate the host before launch.
4. `minivm serve` in `src/counter_api.rs` binds an HTTP listener and exposes `/healthz`, `/count`, and `/incr`.
5. `scripts/linux_setup_bridge.sh` creates a Linux bridge and assigns the host-side bridge address.
6. `minivm build-initramfs` in `src/guest.rs` packages `guest/init` and a BusyBox binary into a `newc` cpio archive.
7. `minivm launch` in `src/launcher.rs` resolves effective launch settings, creates the runtime work directory, computes network allocations, creates TAP interfaces, and asks the configured backend to spawn one guest runtime per VM.

Per-guest launch path:

1. `src/net.rs` computes guest IP, TAP name, and MAC address deterministically from the guest index.
2. `src/launcher.rs` attaches the TAP interface to the named Linux bridge and marks the interface up.
3. `src/backend.rs` selects the requested backend from launch configuration.
4. `src/qemu.rs` builds the QEMU command line with:
   `-kernel`, `-initrd`, `-m`, `-smp 1`, a serial log file, one TAP-backed network device, and kernel command line keys under the `minivm.*` namespace.
5. The QEMU kernel command line provides:
   `minivm.id`, `minivm.guest_ip`, `minivm.gateway`, and `minivm.host_api`.

KVM scaffold path:

1. `src/kvm.rs` can open `/dev/kvm`, query the KVM API version, and attempt `KVM_CREATE_VM`.
2. `src/doctor.rs` uses that probe to validate whether the host can support a future self-hosted KVM backend.
3. `launch --backend kvm` is intentionally not a working runtime path yet; it fails after successful probing because kernel loading, vCPU creation, and device wiring are not implemented.

Guest boot path:

1. Linux boots directly into `/init` from the initramfs.
2. `guest/init` mounts `proc`, `sys`, and `devtmpfs`.
3. `guest/init` parses `/proc/cmdline` for `minivm.id`, `minivm.guest_ip`, `minivm.gateway`, and `minivm.host_api`.
4. `guest/init` configures `lo` and `eth0` using BusyBox `ip`.
5. `guest/init` performs an HTTP request to `minivm.host_api` using BusyBox `wget`.
6. `guest/init` writes status lines to the serial console and powers the VM off.

Host verification path:

1. `src/counter_api.rs` increments an `AtomicU64` on `/incr`.
2. `src/launcher.rs` waits for all backend-provided running-VM handles to complete.
3. Per-VM serial logs are written to the launch work directory.
4. `/count` should equal the number of guests that completed the request path successfully.

Failure boundaries:

- If the bridge does not exist or is misconfigured, TAP creation or guest routing fails.
- If the guest kernel lacks virtio-net support, `eth0` never becomes usable.
- If QEMU is unavailable or cannot use the selected machine or acceleration mode, guest spawn fails in `src/qemu.rs`.
- If the host API is not reachable, guests boot but fail at the HTTP request step.
