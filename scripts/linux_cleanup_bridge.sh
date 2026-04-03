#!/usr/bin/env bash
set -euo pipefail

BRIDGE_NAME="${1:-minivm0}"

if ! ip link show "$BRIDGE_NAME" >/dev/null 2>&1; then
  echo "bridge $BRIDGE_NAME does not exist"
  exit 0
fi

ip link set "$BRIDGE_NAME" down
ip link delete "$BRIDGE_NAME" type bridge

echo "bridge $BRIDGE_NAME removed"

