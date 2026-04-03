#!/usr/bin/env bash
set -euo pipefail

BRIDGE_NAME="${1:-minivm0}"
CIDR="${2:-192.168.100.1/24}"

# This helper creates the smallest possible host bridge for the VM TAP devices.
# Guests will talk to the host API over this network and do not need outbound
# internet or NAT for the first MVP.
if ip link show "$BRIDGE_NAME" >/dev/null 2>&1; then
  echo "bridge $BRIDGE_NAME already exists"
  exit 0
fi

ip link add "$BRIDGE_NAME" type bridge
ip addr add "$CIDR" dev "$BRIDGE_NAME"
ip link set "$BRIDGE_NAME" up

echo "bridge $BRIDGE_NAME is ready at $CIDR"

