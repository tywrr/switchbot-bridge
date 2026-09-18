# switchbot-bridge

A lightweight, zero-dependency Bluetooth Low Energy (BLE) CLI and local REST API server for **SwitchBot** devices, written in Rust.

Built on top of [`btleplug`](https://crates.io/crates/btleplug), `switchbot-bridge` uses native OS Bluetooth stacks (**CoreBluetooth** on macOS Apple Silicon/Intel, and **BlueZ** on Linux).

---

## Features

* **Instant CLI Actuation**: Trigger `press`, `on`, and `off` actions on SwitchBot Bots in milliseconds.
* **Device Discovery**: Discover nearby SwitchBot devices with signal strength (RSSI) and device identifiers.
* **REST API Server**: Run a lightweight HTTP server (`axum`) so home automation platforms like **Home Assistant** can control SwitchBot devices over LAN via simple HTTP POST calls without needing local Bluetooth hardware.
* **Cross-Platform**: Tested on macOS (M1/M2/M3 Apple Silicon and Intel) and Linux.

---

## Installation & Building

Requires Rust 1.75+.

```bash
git clone https://github.com/tywrr/switchbot-bridge.git
cd switchbot-bridge
cargo build --release
```

The compiled binary will be located at `target/release/switchbot-bridge`.

---

## CLI Usage

### 1. Scan for Devices
```bash
# Scan for 5 seconds (default)
switchbot-bridge scan

# Scan for 10 seconds
switchbot-bridge scan --duration 10
```

### 2. Press a Bot
```bash
# Triggers the first SwitchBot Bot found
switchbot-bridge press

# Target a specific Bot by name or UUID substring
switchbot-bridge press --device 6428
switchbot-bridge press --device "WoHand"
```

### 3. Turn On / Turn Off (Switch Mode)
```bash
switchbot-bridge on --device 6428
switchbot-bridge off --device 6428
```

---

## REST API Server (Home Assistant Integration)

Start the HTTP server on port 8080:

```bash
switchbot-bridge serve --port 8080 --host 0.0.0.0
```

### Endpoints

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/health` | Server health check and version status |
| `POST` | `/press` (or `/sauna/press`) | Triggers the **Press** action |
| `POST` | `/on` (or `/sauna/on`) | Triggers the **Turn On** action |
| `POST` | `/off` (or `/sauna/off`) | Triggers the **Turn Off** action |

*(Optional query parameter: `?device=6428` to target a specific Bot).*

### Example Home Assistant Configuration

In Home Assistant's `configuration.yaml`:

```yaml
rest_command:
  sauna_switchbot_press:
    url: "http://m1-13-pr.local:8080/press"
    method: POST
```

---

## License

Dual-licensed under MIT or Apache 2.0.
