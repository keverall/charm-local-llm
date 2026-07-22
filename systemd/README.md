# Systemd Service Files

This directory contains the systemd service definition for Ollama, installed and
managed by `kcharm` (charm-local-llm). `kcharm service install` (or `kcharm start`
on Linux) copies `ollama.service` to `/etc/systemd/system/` and runs
`systemctl daemon-reload` + `systemctl enable ollama`.

On **macOS**, `kcharm` uses direct process management (`ollama serve &`) and does
not use systemd.

## Files

- `ollama.service` — Main service unit (installed to `/etc/systemd/system/`)
- `platform-overrides/` — Drop-in overrides for specific platforms (optional)

## One-time bootstrap

```bash
kcharm service install   # installs unit, passwordless sudo, and desktop autostart
```

This is the replacement for the Ollama repo's `sod.sh` OS-bootstrap steps. After
running it once (with sudo), `make sod` (build + `kcharm start`) handles every
subsequent start-of-day run without the Ollama repo.

## Manual Installation (if not using kcharm)

```bash
sudo cp systemd/ollama.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ollama
```

## Platform Overrides

### NVIDIA GPU (CachyOS)

For GPU acceleration, install the drop-in:

```bash
sudo mkdir -p /etc/systemd/system/ollama.service.d
sudo cp systemd/platform-overrides/cachyos-nvidia.conf /etc/systemd/system/ollama.service.d/
sudo systemctl daemon-reload
```

This adds `DeviceAllow` directives for `/dev/nvidia*` devices.
