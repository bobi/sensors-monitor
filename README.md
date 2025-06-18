# Sensors Monitor

A terminal-based system monitor for Linux, displaying system temperatures, fan speeds, and voltages using the `lm_sensors` utility and a rich TUI.

## Features

- Displays CPU, drive, and other hardware temperatures
- Shows fan speeds and voltages
- Supports custom labels and visibility via config file
- Live updating or one-time snapshot modes
- Color-coded output for easy status recognition
- Uses [Rich](https://github.com/Textualize/rich) for a modern terminal UI

## Requirements

- Python 3.8+
- [lm_sensors](https://github.com/lm-sensors/lm-sensors) installed and available in `$PATH`
- Python packages: `rich`

Install dependencies:
```sh
 pip install -r requirements.txt
```

## Usage

```sh
python -m pylmsensonrs.sm [options]
```

### Options

- `-r`, `--refresh` &lt;seconds&gt;: Refresh rate for live mode (default: 2)
- `-l`, `--live`: Enable live updating (default: off)
- `-1`, `--one-time`: Disable live updating (overrides default)
- `-s`, `--sensors_config` &lt;file&gt;: Path to custom `lm_sensors` config

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

## Signals

Gracefully exits on `SIGINT`, `SIGTERM`, or `SIGHUP`.
