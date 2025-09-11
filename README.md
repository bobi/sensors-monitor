# Sensors Monitor

A terminal-based system monitor for Linux, displaying system temperatures, fan speeds, and voltages using the `lm_sensors` utility and a rich TUI.

## Features

- Displays CPU, drive, and other hardware temperatures
- Shows fan speeds and voltages
- Supports custom labels and visibility via config file
- Color-coded output for easy status recognition
- Uses [ratatui](https://ratatui.rs/) for a modern terminal UI

## Building and Running

### Building

To build the project, use the Cargo build command:

```sh
cargo build
```

To build an optimized release version, use:

```sh
cargo build --release
```

### Running

To run the application, use the Cargo run command:

```sh
cargo run -- -c sensors-monitor-odroid.conf
```

### Options

- `-r`, `--refresh` <seconds>: Refresh interval in seconds (default: 2)
- `-l`, `--lm-sensors-config` lm-sensors config file
- `-j`, `--lm-sensors-json` lm-sensors JSON output file path
- `-c`, `--config` Config file path [default: /etc/sensors-monitor.conf]
- `-h`, `--help` Print help
- `-V`, `--version` Print version

## Configuration

The app reads `/etc/sensors-monitor.conf` for customization. Example:

```ini
[coretemp-isa-0000]
label = CPU
visible = true
hidden_sensoers = temp3

[drivetemp-scsi-0-0]
label = NVMe
visible = false
```

- `label`: Custom chip label
- `visible`: Show/hide chip
- `hidden_sensoers`: Comma-separated list of sensor IDs to hide
