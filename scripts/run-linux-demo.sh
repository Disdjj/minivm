#!/usr/bin/env bash

set -euo pipefail

# This script orchestrates the entire MVP on a Linux host.
# It expects:
# - qemu-system-x86_64
# - a Linux kernel image
# - a static busybox binary
# - root privileges for bridge/TAP setup

COUNT="${COUNT:-3}"
BRIDGE="${BRIDGE:-minivm0}"
BRIDGE_CIDR="${BRIDGE_CIDR:-192.168.100.1/24}"
HOST_API="${HOST_API:-http://192.168.100.1:8080/incr}"
KERNEL_IMAGE="${KERNEL_IMAGE:-}"
BUSYBOX_BIN="${BUSYBOX_BIN:-}"

if [[ -z "${KERNEL_IMAGE}" ]]; then
  echo "set KERNEL_IMAGE=/path/to/vmlinux" >&2
  exit 1
fi

if [[ -z "${BUSYBOX_BIN}" ]]; then
  echo "set BUSYBOX_BIN=/path/to/static-busybox" >&2
  exit 1
fi

mkdir -p runtime

echo "[1/4] building initramfs"
cargo run -- build-initramfs --busybox "${BUSYBOX_BIN}" --output runtime/initramfs.cpio

echo "[2/4] preparing bridge"
sudo ./scripts/linux_setup_bridge.sh "${BRIDGE}" "${BRIDGE_CIDR}"

echo "[3/4] starting counter API"
HOST_LISTEN="${HOST_API#http://}"
HOST_LISTEN="${HOST_LISTEN%/incr}"
cargo run -- serve --listen "${HOST_LISTEN}" &
API_PID=$!

cleanup() {
  kill "${API_PID}" 2>/dev/null || true
  sudo ./scripts/linux_cleanup_bridge.sh "${BRIDGE}" || true
}
trap cleanup EXIT

sleep 1

echo "[4/4] launching ${COUNT} guests"
sudo cargo run -- launch \
  --count "${COUNT}" \
  --kernel "${KERNEL_IMAGE}" \
  --initramfs runtime/initramfs.cpio \
  --bridge "${BRIDGE}" \
  --host-api "${HOST_API}"

echo "final counter value:"
curl -s "${HOST_API%/incr}/count"
echo
