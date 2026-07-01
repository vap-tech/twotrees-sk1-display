#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET="${TARGET:-aarch64-unknown-linux-musl}"
DIST_DIR="$CRATE_DIR/dist/$TARGET"
ARCHIVE="$CRATE_DIR/dist/vaptechclient-$TARGET.tar.gz"

cd "$CRATE_DIR"

echo "Building vaptechclient for target: $TARGET"
if [[ "$TARGET" == "aarch64-unknown-linux-musl" && -z "${RUSTFLAGS:-}" ]]; then
    export RUSTFLAGS="-C linker=rust-lld"
fi

CARGO_BIN="${CARGO:-cargo}"
if [[ -x "$HOME/.cargo/bin/cargo" && -z "${CARGO:-}" ]]; then
    CARGO_BIN="$HOME/.cargo/bin/cargo"
fi

"$CARGO_BIN" build --release --target "$TARGET"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

install -m 0755 "$CRATE_DIR/target/$TARGET/release/vaptechclient" "$DIST_DIR/vaptechclient"
install -m 0644 "$SCRIPT_DIR/config.printer.toml" "$DIST_DIR/config.toml"
install -m 0644 "$SCRIPT_DIR/vaptechclient.service" "$DIST_DIR/vaptechclient.service"

tar -czf "$ARCHIVE" -C "$DIST_DIR" .

echo "Prepared:"
echo "  $DIST_DIR/vaptechclient"
echo "  $DIST_DIR/config.toml"
echo "  $DIST_DIR/vaptechclient.service"
echo "  $ARCHIVE"
