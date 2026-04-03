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
- `guest/init`: the guest workload that configures `eth0`, calls the host API, and powers off
- `scripts/linux_setup_bridge.sh`: helper for creating a host bridge used by the TAP interfaces

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

2. Create a bridge that the host API will listen on:

```bash
sudo ./scripts/linux_setup_bridge.sh minivm0 192.168.100.1/24
```

3. Build the initramfs:

```bash
cargo run -- build-initramfs \
  --busybox /usr/bin/busybox \
  --output out/initramfs.cpio
```

4. Start the counter API:

```bash
cargo run -- serve --listen 192.168.100.1:8080
```

5. Launch three guests:

```bash
sudo cargo run -- launch \
  --count 3 \
  --kernel /path/to/bzImage \
  --initramfs out/initramfs.cpio \
  --bridge minivm0 \
  --host-api http://192.168.100.1:8080/incr
```

6. Verify the host count:

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
- QEMU is wrapped behind a small launcher module. That keeps the orchestration logic reusable when we replace QEMU with a custom KVM-based backend later.

