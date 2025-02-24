#!/usr/bin/env python3
import configparser
import json
import optparse
import os
import re
import signal
import subprocess
import sys
from threading import Event
from typing import Optional

from rich import box
from rich.align import Align
from rich.console import Console, RenderableType, Group
from rich.layout import Layout
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

ADAPTER_PROP = "Adapter"

console = Console()
console.show_cursor(False)

running = Event()

CONFIG_FILE = "/etc/sensors-monitor.conf"

chip_sort_order = {
    re.compile("^coretemp-.*"): 1,
    re.compile("^drivetemp-.*"): 2,
    re.compile("^nvme-.*"): 3,
    re.compile("^acpitz-.*"): 4,
}


class Sensoer:
    chip_id: str
    sensor_id: str
    default_chip_label: str
    chip_label: str
    sensor_label: str
    chip_order: int

    def __init__(self, chip_id: str, sensoer_id: str, default_chip_label: Optional[str]):
        self.chip_id = chip_id
        self.sensor_id = sensoer_id
        self.default_chip_label = default_chip_label
        self.chip_label = get_custom_chip_label(chip_id)
        self.sensor_label = get_custom_sensor_label(chip_id, sensoer_id)
        self.chip_order = get_chip_order(chip_id)


class FanSpeed(Sensoer):
    value: Optional[float]
    min: Optional[float]

    def __init__(self, chip_id: str, sensoer_id: str, default_chip_label: Optional[str]):
        super().__init__(chip_id, sensoer_id, default_chip_label)
        self.value = None
        self.min = None


class Voltage(Sensoer):
    value: Optional[float]
    min: Optional[float]
    max: Optional[float]

    def __init__(self, chip_id: str, sensoer_id: str, default_chip_label: Optional[str]):
        super().__init__(chip_id, sensoer_id, default_chip_label)
        self.value = None
        self.min = None
        self.max = None


class Temp(Sensoer):
    value: Optional[float]
    high: Optional[float]
    critical: Optional[float]

    def __init__(self, chip_id: str, sensoer_id: str, default_chip_label: Optional[str]):
        super().__init__(chip_id, sensoer_id, default_chip_label)
        self.value = None
        self.high = None
        self.critical = None


class HddTemp(Temp):
    lowest: Optional[float]
    highest: Optional[float]

    def __init__(self, chip_id: str, sensoer_id: str, default_chip_label: Optional[str]):
        super().__init__(chip_id, sensoer_id, default_chip_label)
        self.lowest = None
        self.highest = None


class SensorsData:
    volts: list[Voltage]
    temps: list[Temp]
    hdd_temps: list[HddTemp]
    fans: list[Voltage]
    raw_data: dict

    def __init__(self, raw_data: dict):
        self.raw_data = raw_data
        self.volts = []
        self.temps = []
        self.hdd_temps = []
        self.fans = []


def get_chip_order(chip_id):
    for key, value in chip_sort_order.items():
        if key.match(chip_id) is not None:
            return value

    return sys.maxsize - 1


def load_config():
    config_parser = configparser.ConfigParser()

    config_path = CONFIG_FILE

    if os.path.exists(config_path):
        try:
            config_parser.read(config_path)
            return config_parser
        except FileNotFoundError:
            pass
        except Exception as e:
            console.log(f"[red]Error: Failed to read {CONFIG_FILE} - {e}[/red]")
    return config_parser


config = load_config()


def get_sensors_json(lm_config: str):
    try:
        if lm_config is None:
            lm_config = "/dev/null"

        result = subprocess.run(["sensors", "-c", lm_config, "-j"], capture_output=True, text=True, check=True)

        return json.loads(result.stdout)
    except FileNotFoundError:
        console.log("[red]Error: lm_sensors is not installed![/red]")
        exit(1)
    except subprocess.CalledProcessError:
        console.log("[red]Error: Failed to run sensors command![/red]")
        exit(1)
    except json.JSONDecodeError:
        console.log("[red]Error: Failed to parse JSON output![/red]")
        exit(1)


def get_custom_sensor_label(chip_id, sensor_id):
    if chip_id in config and sensor_id in config[chip_id]:
        return f"{config[chip_id][sensor_id]}"
    return sensor_id


def get_custom_chip_label(chip_id):
    if chip_id in config and "label" in config[chip_id]:
        return f"{config[chip_id]['label']}"

    return chip_id


def is_chip_visible(chip_id):
    if chip_id in config and "visible" in config[chip_id]:
        return config.getboolean(chip_id, "visible")
    return True


def is_sensoer_visible(chip_id, sensor_id):
    if chip_id in config and "hidden_sensoers" in config[chip_id]:
        hidden_sensoers = config[chip_id]["hidden_sensoers"].split(",")
        if sensor_id in hidden_sensoers:
            return False
    return True


def default_str_if_none(s, default_str: str = None):
    if s is None:
        return "" if default_str is None else default_str
    return str(s)


def parse_sensors_json(sensors_json: dict) -> SensorsData:
    output: SensorsData = SensorsData(sensors_json)

    for chip_id, chip_data in sensors_json.items():
        if not is_chip_visible(chip_id):
            continue

        adapter: Optional[str] = None
        if ADAPTER_PROP in chip_data:
            adapter = f"{chip_data[ADAPTER_PROP]}"

        for sensor_id, sensoer_values in chip_data.items():
            if not isinstance(sensoer_values, dict) or not is_sensoer_visible(chip_id, sensor_id):
                continue

            temps: dict = {}
            hdd_temps: dict = {}
            volts: dict = {}
            fans: dict = {}

            for name, value in sensoer_values.items():
                if name.startswith("temp"):
                    if chip_id.startswith("drivetemp"):
                        if name.endswith("_input"):
                            hdd_temps.setdefault(sensor_id, HddTemp(chip_id, sensor_id, adapter)).value = value
                        elif name.endswith("_max"):
                            hdd_temps.setdefault(sensor_id, HddTemp(chip_id, sensor_id, adapter)).high = value
                        elif name.endswith("_crit"):
                            hdd_temps.setdefault(sensor_id, HddTemp(chip_id, sensor_id, adapter)).critical = value
                        elif name.endswith("_lowest"):
                            hdd_temps.setdefault(sensor_id, HddTemp(chip_id, sensor_id, adapter)).lowest = value
                        elif name.endswith("_highest"):
                            hdd_temps.setdefault(sensor_id, HddTemp(chip_id, sensor_id, adapter)).highest = value
                    else:
                        if name.endswith("_input"):
                            temps.setdefault(sensor_id, Temp(chip_id, sensor_id, adapter)).value = value
                        elif name.endswith("_max"):
                            temps.setdefault(sensor_id, Temp(chip_id, sensor_id, adapter)).high = value
                        elif name.endswith("_crit"):
                            temps.setdefault(sensor_id, Temp(chip_id, sensor_id, adapter)).critical = value

                elif name.startswith("fan"):
                    if name.endswith("_input"):
                        fans.setdefault(sensor_id, FanSpeed(chip_id, sensor_id, adapter)).value = value
                    elif name.endswith("_min"):
                        fans.setdefault(sensor_id, FanSpeed(chip_id, sensor_id, adapter)).min = value

                elif name.startswith("in"):
                    if name.endswith("_input"):
                        volts.setdefault(sensor_id, Voltage(chip_id, sensor_id, adapter)).value = value
                    elif name.endswith("_min"):
                        volts.setdefault(sensor_id, Voltage(chip_id, sensor_id, adapter)).min = value
                    elif name.endswith("_max"):
                        volts.setdefault(sensor_id, Voltage(chip_id, sensor_id, adapter)).max = value

            output.temps.extend([*temps.values()])
            output.hdd_temps.extend([*hdd_temps.values()])
            output.volts.extend([*volts.values()])
            output.fans.extend([*fans.values()])

    output.temps = sorted(output.temps, key=lambda item: item.chip_order)
    output.hdd_temps = sorted(output.hdd_temps, key=lambda item: item.chip_order)
    output.volts = sorted(output.volts, key=lambda item: item.chip_order)
    output.fans = sorted(output.fans, key=lambda item: item.chip_order)

    return output


def get_colored_temp(temp, high):
    if temp is None:
        return Text("")

    if high is None:
        high = sys.maxsize

    if temp >= high * .8:
        return Text(f"{temp}°C", style="bold red")
    elif temp >= high * .6:
        return Text(f"{temp}°C", style="yellow")
    else:
        return Text(f"{temp}°C", style="green")


def get_colored_fan(speed):
    if speed is None:
        return Text("")

    return Text(f"{speed} RPM", style="green")


def get_colored_voltage(voltage, min_voltage, max_voltage):
    if voltage is None:
        return Text("")

    if min_voltage is None:
        min_voltage = -sys.maxsize - 1

    if max_voltage is None:
        max_voltage = sys.maxsize

    if voltage < min_voltage:
        return Text(f"{voltage:.2f}V", style="yellow")
    elif voltage > max_voltage:
        return Text(f"{voltage:.2f}V", style="red")
    else:
        return Text(f"{voltage:.2f}V", style="green")


def build_temp_table(temps: [Temp]) -> Optional[Table]:
    if not temps:
        return None

    table = Table(box=box.MINIMAL)
    table.add_column("Chip / Sensor", style="magenta", justify="left")
    table.add_column("Current", style="bold", justify="right")
    table.add_column("High", style="dim", justify="right")
    table.add_column("Critical", style="dim", justify="right")

    last_chip_id: Optional[str] = None
    for sensor_value in temps:
        assert isinstance(sensor_value, Temp)

        if last_chip_id is not None and last_chip_id != sensor_value.chip_id:
            table.add_section()

        table.add_row(
            f"{sensor_value.chip_label} {sensor_value.sensor_label}",
            get_colored_temp(sensor_value.value, sensor_value.high),
            default_str_if_none(sensor_value.high),
            default_str_if_none(sensor_value.critical),
        )

        last_chip_id = sensor_value.chip_id

    return table


def build_hdd_temp_table(temps: [HddTemp]) -> Optional[Table]:
    if not temps:
        return None

    table = Table(box=box.MINIMAL)
    table.add_column("Drive", style="magenta", justify="left")
    table.add_column("Current", style="bold", justify="right")
    table.add_column("High", style="dim", justify="right")
    table.add_column("Critical", style="dim", justify="right")
    table.add_column("Lowest", style="dim", justify="right")
    table.add_column("Highest", style="dim", justify="right")

    for sensor_value in temps:
        assert isinstance(sensor_value, HddTemp)

        table.add_row(
            f"{sensor_value.chip_label} {sensor_value.sensor_label}",
            get_colored_temp(sensor_value.value, sensor_value.high),
            default_str_if_none(sensor_value.high),
            default_str_if_none(sensor_value.critical),
            default_str_if_none(sensor_value.lowest),
            default_str_if_none(sensor_value.highest),
        )

    return table


def build_voltage_table(voltages: [Voltage]) -> Optional[Table]:
    if not voltages:
        return None

    table = Table(box=box.MINIMAL)
    table.add_column("Chip / Sensor", style="magenta", justify="left")
    table.add_column("Current", style="bold", justify="right")
    table.add_column("Min", style="dim", justify="right")
    table.add_column("Max", style="dim", justify="right")

    last_chip_id: Optional[str] = None
    for sensor_value in voltages:
        assert isinstance(sensor_value, Voltage)

        if last_chip_id is not None and last_chip_id != sensor_value.chip_id:
            table.add_section()

        table.add_row(
            f"{sensor_value.chip_label} {sensor_value.sensor_label}",
            get_colored_voltage(sensor_value.value, sensor_value.min, sensor_value.max),
            default_str_if_none(sensor_value.min),
            default_str_if_none(sensor_value.max),
        )

        last_chip_id = sensor_value.chip_id

    return table


def build_fans_table(fans: [FanSpeed]) -> Optional[Table]:
    if not fans:
        return None

    table = Table(box=box.MINIMAL)
    table.add_column("Fan", style="magenta", justify="left")
    table.add_column("Current", style="bold", justify="right")
    table.add_column("Min", style="dim", justify="right")

    last_chip_id: Optional[str] = None
    for sensor_value in fans:
        assert isinstance(sensor_value, FanSpeed)

        if last_chip_id is not None and last_chip_id != sensor_value.chip_id:
            table.add_section()

        table.add_row(
            f"{sensor_value.chip_label} {sensor_value.sensor_label}",
            get_colored_fan(sensor_value.value),
            default_str_if_none(sensor_value.min),
        )

        last_chip_id = sensor_value.chip_id

    return table


def build_sensors_ui(lm_config: str) -> RenderableType:
    sensors_json = get_sensors_json(lm_config)

    sensors_data: SensorsData = parse_sensors_json(sensors_json)

    left_tables = []
    main_tables = []
    right_tables = []

    if table := build_temp_table(sensors_data.temps):
        left_tables.append(table)
    if table := build_hdd_temp_table(sensors_data.hdd_temps):
        left_tables.append(table)
    if table := build_voltage_table(sensors_data.volts):
        main_tables.append(table)
    if table := build_fans_table(sensors_data.fans):
        right_tables.append(table)

    layout = Layout()

    layout.split_row(Layout(name="lside"), Layout(name="main"), Layout(name="rside"))

    if left_tables:
        layout["lside"].update(Panel(
            Align.center(Group(*left_tables)),
            title="[bold cyan]Temperatures[/bold cyan]")
        )
    else:
        layout["lside"].visible = False
    if main_tables:
        layout["main"].update(Panel(
            Align.center(Group(*main_tables)),
            title="[bold cyan]Voltages[/bold cyan]"
        ))
    else:
        layout["main"].visible = False
    if right_tables:
        layout["rside"].update(Panel(
            Align.center(Group(*right_tables)),
            title="[bold cyan]Fans[/bold cyan]"
        ))
    else:
        layout["rside"].visible = False

    return layout


def monitor_sensors(live: bool, refresh_rate: int, lm_config: str):
    global running

    if not live:
        console.print(build_sensors_ui(lm_config))
    else:
        with Live(console=console, auto_refresh=False, screen=True) as live:
            while not running.is_set():
                live.update(build_sensors_ui(lm_config), refresh=True)
                running.wait(refresh_rate)


def handle_exit(_signum, _frame):
    global running
    running.set()
    exit(0)


if __name__ == "__main__":
    parser = optparse.OptionParser(description="Monitor system temperatures, fan speeds, and voltages.")
    parser.add_option("-r", "--refresh", type=int, default=2, help="Refresh rate in seconds (default: 2)")
    parser.add_option("-l", "--live", action="store_true", default=False, help="Live updates")
    parser.add_option("-s", "--sensors_config", type=str, help="Custom lm-sensoers config")
    options, args = parser.parse_args()

    signal.signal(signal.SIGINT, handle_exit)
    signal.signal(signal.SIGTERM, handle_exit)
    signal.signal(signal.SIGHUP, handle_exit)

    monitor_sensors(options.live, options.refresh, options.sensors_config)
