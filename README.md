# minivm

`minivm` is a deliberately small prototype for the workflow we discussed:

- start `N` lightweight VMs on a Linux host
- give each guest network access through a TAP-backed interface
- boot an initramfs whose `/init` script calls a host-side HTTP API
- verify that the host counter reaches `N`

This repository does **not** implement a full Firecracker-like VMM yet. The first working cut uses `qemu-system-x86_64` as the hypervisor backend so we can prove the control plane, guest payload, and networking end to end before replacing QEMU with a custom KVM runner.

## What Is Included

- `serve`: a small counter API with `/incr`, `/count`, and `/healthz`
- `build-initramfs`: packages a BusyBox binary and the guest `/init` script into a Linux initramfs
- `launch`: spawns `N` QEMU guests, allocates deterministic TAP/IP/MAC values, and waits for all guests to finish
- `init`: interactive terminal wizard for generating `minivm.toml`
- `doctor`: validates the current host and configuration before launch
- `guest/init`: the guest workload that configures `eth0`, calls the host API, and powers off
- `scripts/linux_setup_bridge.sh`: helper for creating a host bridge used by the TAP interfaces
- `minivm.toml.example`: example config file for storing shared defaults

## Backend Status

- `qemu`: working backend used for the current end-to-end MVP
- `kvm`: first self-hosted backend scaffold; it currently probes `/dev/kvm` and reserves the backend seam, but it does not boot guests yet

## Host Requirements

The runtime path targets **Linux x86_64 with KVM**:

- `/dev/kvm`
- `qemu-system-x86_64`
- `ip` from `iproute2`
- a Linux kernel image with virtio-net, devtmpfs, procfs, and sysfs enabled
- a static BusyBox binary to embed into the initramfs

The current development machine for this repo is macOS/arm64, so the repository only receives compile-level validation locally. The actual VM launch flow needs to be run on Linux.

## Quick Start On Linux

1. Build the binary:

```bash
cargo build
```

2. Generate a config file with the interactive wizard:

```bash
cargo run -- init
```

If you prefer a static template instead:

```bash
cp minivm.toml.example minivm.toml
```

3. Run environment checks:

```bash
cargo run -- doctor
```

4. Create a bridge that the host API will listen on:

```bash
sudo ./scripts/linux_setup_bridge.sh minivm0 192.168.100.1/24
```

5. Build the initramfs:

```bash
cargo run -- build-initramfs \
  --busybox /usr/bin/busybox \
  --output out/initramfs.cpio
```

6. Start the counter API:

```bash
cargo run -- serve --listen 192.168.100.1:8080
```

7. Launch three guests:

```bash
sudo ./target/debug/minivm launch \
  --count 3 \
  --kernel /path/to/bzImage \
  --initramfs out/initramfs.cpio \
  --bridge minivm0 \
  --host-api http://192.168.100.1:8080/incr
```

8. Verify the host count:

```bash
curl http://192.168.100.1:8080/count
```

The expected result is:

```text
3
```

## Design Notes

- Guests use one vCPU and a small amount of RAM because the only workload is a single HTTP request.
- The initramfs is intentionally tiny. There is no root disk and no block device in this first cut.
- TAP setup is done on the host so that each guest gets a normal `virtio-net` device connected to the bridge.
- The launcher now works through a backend trait that returns a generic running-VM handle rather than a raw process handle. This is the seam intended for the future in-process KVM VMM.
- QEMU remains the only backend that can boot guests today.
- Shared defaults can live in `minivm.toml`, while command-line flags still override them.
- `init` is meant to be the easiest on-ramp for repeated local experiments; it pre-fills prompts from the current config file and from common host defaults when possible.
