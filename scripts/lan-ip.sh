#!/bin/sh
# Prints the address a phone on the same network can actually reach.
#
# `hostname -I` is no good here: it lists every docker bridge and VPN address
# in arbitrary order. What we want is the source IP the kernel picks to talk
# to the default gateway, which is by definition on the LAN the phone shares.
set -eu

gw=$(ip route show default | awk '{print $3; exit}')
[ -n "$gw" ] || { echo "no default route" >&2; exit 1; }

ip route get "$gw" | awk '{for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit }}'
