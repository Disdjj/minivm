# llmdoc index

This directory is the primary source of truth for the current `minivm` prototype.

Read order:

1. `llmdoc/overview/project-overview.md`
   Summary of project goal, current scope, and explicit non-goals.

2. `llmdoc/architecture/runtime-flow.md`
   End-to-end runtime path from host API startup to guest request completion.

3. `llmdoc/architecture/module-map.md`
   Code ownership map for the current Rust modules and shell helpers.

4. `llmdoc/guides/run-on-linux.md`
   Operational guide for building, launching, and verifying the MVP on Linux.

5. `llmdoc/reference/cli.md`
   Stable description of the current CLI subcommands and important flags.

Current status:

- The repository implements a Linux-first microVM MVP.
- The only backend that can boot guests today is QEMU with optional KVM acceleration.
- A first `kvm` backend scaffold now exists for host probing and backend integration, but it does not boot guests yet.
- The validated workflow is: host counter API up, initramfs generated, `N` guests launched, each guest performs one HTTP request to `/incr`, guests exit.
- The CLI now supports optional `minivm.toml` configuration loading and a `doctor` command for environment checks.
- The CLI now also supports an interactive `init` wizard for generating `minivm.toml`.

Key source files:

- `src/main.rs`
- `src/cli.rs`
- `src/config.rs`
- `src/backend.rs`
- `src/counter_api.rs`
- `src/doctor.rs`
- `src/guest.rs`
- `src/kvm.rs`
- `src/net.rs`
- `src/qemu.rs`
- `src/wizard.rs`
- `src/launcher.rs`
- `guest/init`
- `minivm.toml.example`
- `scripts/linux_setup_bridge.sh`
- `scripts/linux_cleanup_bridge.sh`
- `scripts/run-linux-demo.sh`
