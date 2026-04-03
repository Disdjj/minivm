# module map

Entrypoint:

- `src/main.rs`
  Initializes tracing and dispatches to the CLI.

- `src/cli.rs`
  Defines the public subcommands and their arguments.
  This file is the canonical source for user-facing CLI behavior.

- `src/config.rs`
  Loads optional TOML configuration from `minivm.toml`.
  Owns the file format for shared defaults.

Host services:

- `src/counter_api.rs`
  Implements the host-side HTTP counter service.
  Owns `/healthz`, `/count`, and `/incr`.

- `src/doctor.rs`
  Implements host environment checks.
  Owns diagnostics for Linux, `/dev/kvm`, QEMU, `ip`, bridge presence, and configured paths.

Guest image creation:

- `src/guest.rs`
  Builds the initramfs archive in pure Rust.
  Embeds the default guest init script from `guest/init`.
  Owns the minimal `newc` cpio writer.

- `guest/init`
  Implements the whole guest workload for the MVP.
  Owns guest-side filesystem setup, kernel argument parsing, network configuration, HTTP request, and shutdown behavior.

Network allocation and orchestration:

- `src/net.rs`
  Allocates gateway IP, guest IPs, TAP names, and MAC addresses deterministically.

- `src/launcher.rs`
  Orchestrates multi-guest launch.
  Owns TAP lifecycle, runtime directory creation, QEMU child process spawning, and exit waiting.

Hypervisor backend:

- `src/backend.rs`
  Defines the backend abstraction and backend factory.
  This is the new seam between orchestration and hypervisor implementation.

- `src/qemu.rs`
  Implements the current `qemu` backend and translates per-guest launch specifications into a QEMU command line.
  This module should be replaceable without changing CLI or orchestration behavior.

Operational helpers:

- `scripts/linux_setup_bridge.sh`
  Creates a Linux bridge and assigns the host-side address.

- `scripts/linux_cleanup_bridge.sh`
  Removes the Linux bridge created for the MVP.

- `scripts/run-linux-demo.sh`
  Runs the common workflow end to end for Linux environments that already satisfy the host requirements.

Replacement strategy:

- Keep `src/cli.rs`, `src/config.rs`, `src/counter_api.rs`, `src/doctor.rs`, `src/net.rs`, `src/launcher.rs`, and `guest/init` stable if possible.
- Treat `src/backend.rs` as the replacement seam and `src/qemu.rs` as the current implementation behind that seam.
