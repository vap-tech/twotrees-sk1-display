#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

TARGET="${TARGET:-aarch64-unknown-linux-musl}"
PRINTER_HOST="${1:-${PRINTER_HOST:-192.168.0.20}}"
PRINTER_USER="${PRINTER_USER:-mks}"
INSTALL_CONFIG="${INSTALL_CONFIG:-0}"
SUDO_PASSWORD="${SUDO_PASSWORD:-${SSHPASS:-}}"
DIST_DIR="$CRATE_DIR/dist/$TARGET"
REMOTE="$PRINTER_USER@$PRINTER_HOST"

if [[ ! -x "$DIST_DIR/vaptechclient" ]]; then
    echo "Missing $DIST_DIR/vaptechclient"
    echo "Run first: TARGET=$TARGET ./packaging/build-release.sh"
    exit 1
fi

SSH=(ssh -o StrictHostKeyChecking=no)
SCP=(scp -o StrictHostKeyChecking=no)

if [[ -n "${SSHPASS:-}" ]]; then
    SSH=(sshpass -p "$SSHPASS" "${SSH[@]}")
    SCP=(sshpass -p "$SSHPASS" "${SCP[@]}")
fi

echo "Uploading vaptechclient to $REMOTE"
"${SCP[@]}" "$DIST_DIR/vaptechclient" "$REMOTE:/tmp/vaptechclient.bin"
"${SCP[@]}" "$DIST_DIR/config.toml" "$REMOTE:/tmp/vaptechclient.config.toml"
"${SCP[@]}" "$DIST_DIR/vaptechclient.service" "$REMOTE:/tmp/vaptechclient.service"

shell_quote() {
    printf "%q" "$1"
}

echo "Installing on printer"
"${SSH[@]}" "$REMOTE" \
    "INSTALL_CONFIG=$(shell_quote "$INSTALL_CONFIG") SUDO_PASSWORD=$(shell_quote "$SUDO_PASSWORD") bash -s" <<'REMOTE_SCRIPT'
set -euo pipefail

run_sudo() {
    if [ -n "${SUDO_PASSWORD:-}" ]; then
        printf '%s\n' "$SUDO_PASSWORD" | sudo -S "$@"
    else
        sudo "$@"
    fi
}

run_sudo install -d -m 0755 /etc/vaptechclient /var/lib/vaptechclient /tmp/vaptechclient/thumbnails
run_sudo install -m 0755 /tmp/vaptechclient.bin /usr/local/bin/vaptechclient
run_sudo chown -R mks:mks /var/lib/vaptechclient /tmp/vaptechclient

if [ ! -f /etc/vaptechclient/config.toml ] || [ "${INSTALL_CONFIG:-0}" = "1" ]; then
    run_sudo install -m 0644 /tmp/vaptechclient.config.toml /etc/vaptechclient/config.toml
else
    echo "Keeping existing /etc/vaptechclient/config.toml"
    echo "Set INSTALL_CONFIG=1 on the remote command if you intentionally want to overwrite it."
fi

run_sudo install -m 0644 /tmp/vaptechclient.service /etc/systemd/system/vaptechclient.service
run_sudo systemctl daemon-reload
run_sudo systemctl enable vaptechclient.service
run_sudo systemctl restart vaptechclient.service
run_sudo systemctl --no-pager --full status vaptechclient.service
REMOTE_SCRIPT
