# vaptechclient packaging

This directory contains the lightweight printer deployment path.

The printer should not build Rust code. Build the release binary on a stronger
machine, then copy only:

- `vaptechclient`
- `/etc/vaptechclient/config.toml`
- `vaptechclient.service`

## Build

Default target is static `aarch64-unknown-linux-musl`:

```bash
./packaging/build-release.sh
```

For a native test build on the current machine:

```bash
TARGET="$(rustc -vV | sed -n 's/^host: //p')" ./packaging/build-release.sh
```

Output goes to:

```text
vaptechclient/dist/<target>/
vaptechclient/dist/vaptechclient-<target>.tar.gz
```

## Deploy

```bash
./packaging/deploy.sh 192.168.0.20
```

For password-based SSH:

```bash
SSHPASS='M342R_crb' ./packaging/deploy.sh 192.168.0.20
```

If the sudo password differs from the SSH password:

```bash
SSHPASS='...' SUDO_PASSWORD='...' ./packaging/deploy.sh 192.168.0.20
```

Environment knobs:

```bash
TARGET=aarch64-unknown-linux-musl
PRINTER_USER=mks
PRINTER_HOST=192.168.0.20
SSHPASS=...
SUDO_PASSWORD=...
```

The deploy script does not overwrite an existing printer config by default.
Edit `/etc/vaptechclient/config.toml` on the printer after first install,
especially the `[hmi].serial` path.

## Service

```bash
sudo systemctl status vaptechclient
sudo journalctl -u vaptechclient -f
sudo systemctl restart vaptechclient
```
