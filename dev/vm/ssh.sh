#!/usr/bin/env bash
root=$(cd "$(dirname "$0")/../.." && pwd)
exec ssh -q -i "$root/.vm/id_ed25519" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=5 -p 10022 root@127.0.0.1 "$@"
