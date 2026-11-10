#!/usr/bin/env bash
# Build mmm-native for the Pi and install it as /opt/mmm-native/mmm-native, keeping the
# previous binary as mmm-native.prev, then restart the service.
#
#   native/deploy/deploy.sh            # host "pi" from ~/.ssh/config
#   MMM_PI=user@host native/deploy/deploy.sh
#
# Needs: cargo-zigbuild + zig, the aarch64-unknown-linux-gnu rustup target (see
# ROADMAP.md), and the one-time install in deploy/README.md (/opt/mmm-native owned by
# max, the unit installed). Rollback: see README.md.
set -euo pipefail

HOST="${MMM_PI:-pi}"
TARGET=aarch64-unknown-linux-gnu
DIR=/opt/mmm-native
CRATE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$CRATE/target/$TARGET/release/mmm-native"

run() {
    printf '+ %s\n' "$*"
    "$@"
}

echo "== build (aarch64, glibc 2.36)"
cd "$CRATE"
run cargo zigbuild --release --target "$TARGET.2.36"

echo "== check dynamic dependencies (only libc/libm expected)"
if command -v readelf >/dev/null; then
    needed="$(readelf -d "$BIN" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p' | sort | tr '\n' ' ')"
    echo "NEEDED: $needed"
    for lib in $needed; do
        case "$lib" in
            libc.so.6 | libm.so.6 | ld-linux-aarch64.so.1) ;;
            *) echo "unexpected shared library $lib; not deploying" >&2; exit 1 ;;
        esac
    done
else
    echo "readelf not found; skipping"
fi
ls -l "$BIN"

# A temp name in the target directory, so the final mv is a same-filesystem rename
# (atomic: the service never sees a half-copied binary).
TMP="$DIR/.mmm-native.new.$$"
echo "== copy to $HOST:$TMP"
run scp "$BIN" "$HOST:$TMP"

echo "== install on $HOST (previous binary kept as $DIR/mmm-native.prev)"
# ln -f: .prev becomes a second name for the current binary (no copy, replaces the
# older .prev); mv -f: atomic rename of the new binary over the current name.
run ssh "$HOST" "set -eu
chmod 755 '$TMP'
if [ -e '$DIR/mmm-native' ]; then ln -f '$DIR/mmm-native' '$DIR/mmm-native.prev'; fi
mv -f '$TMP' '$DIR/mmm-native'
ls -l '$DIR'"

echo "== restart"
run ssh -t "$HOST" sudo systemctl restart mmm-native
run ssh "$HOST" systemctl --no-pager --lines=5 status mmm-native || true
echo "done. Logs: ssh $HOST journalctl -u mmm-native -f"
