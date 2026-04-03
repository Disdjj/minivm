# cli reference

Binary:

- `minivm`

Subcommands:

- `init`
  Source of truth: `src/cli.rs`, `src/wizard.rs`, `src/config.rs`
  Purpose: generate a config file interactively.
  Important arguments: `--output`, `--force`

- `serve`
  Source of truth: `src/cli.rs`, `src/counter_api.rs`
  Purpose: start the host-side counter API.
  Important argument: `--listen`

- `build-initramfs`
  Source of truth: `src/cli.rs`, `src/guest.rs`
  Purpose: create an initramfs archive containing BusyBox and `/init`.
  Important arguments: `--busybox`, `--output`, `--init-script`

- `launch`
  Source of truth: `src/cli.rs`, `src/launcher.rs`, `src/backend.rs`, `src/qemu.rs`, `src/kvm.rs`
  Purpose: create TAP devices, select the configured backend, start `N` guests, and wait for them to exit.
  Important arguments:
  `--count`
  `--kernel`
  `--initramfs`
  `--workdir`
  `--host-api`
  `--bridge`
  `--subnet-base`
  `--prefix-len`
  `--memory-mib`
  `--qemu-bin`
  `--machine`
  `--accel`
  `--tap-prefix`
  `--skip-tap-setup`
  `--keep-taps`

- `doctor`
  Source of truth: `src/cli.rs`, `src/doctor.rs`, `src/config.rs`
  Purpose: validate the loaded configuration and the local host environment.
  Important argument: `--strict`

- `print-guest-init`
  Source of truth: `src/cli.rs`, `guest/init`
  Purpose: print the embedded default guest `/init` script for inspection or customization.

Current semantics:

- `init` writes a TOML config file using interactive terminal prompts.
- `init` pre-fills values from the current loaded config and from common host defaults when possible.
- `serve` keeps all state in memory. Counter state resets on process restart.
- `build-initramfs` emits an uncompressed `newc` cpio archive.
- A global `--config` flag can load defaults from `minivm.toml`.
- `launch` uses one vCPU per guest.
- `launch` resolves config-file defaults first and then applies CLI overrides.
- `launch` supports `qemu` as the working backend.
- `launch` recognizes `kvm`, but that backend currently stops after host probing and returns a not-implemented error.
- `launch` uses one QEMU process per guest when the backend is `qemu`.
- `launch` deletes created TAP devices on exit unless `--keep-taps` is set.
- `launch` writes one serial log file per guest under the work directory.
- `doctor` reports pass, warn, and fail outcomes without mutating the system.

Stability notes:

- The CLI is stable for the current MVP.
- The backend implementation behind `launch` is expected to change when QEMU is replaced with a custom VMM.
- The guest kernel command line contract under the `minivm.*` namespace is currently part of the runtime ABI between `src/qemu.rs` and `guest/init`.
