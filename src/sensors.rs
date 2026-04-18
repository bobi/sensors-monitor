use crate::config::SmConfig;
use color_eyre::eyre::bail;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::io::ErrorKind;
use std::process::Command;

const NULL_DEVICE: &str = "/dev/null";
// some drives report unrealistically high "max" values that would break the gauge scale
const DRIVE_TEMP_MAX_CAP: f64 = 150.0;

#[derive(Debug, Deserialize, Clone)]
pub struct Temp {
    pub chip_id: String,
    pub chip_label: String,
    pub sensor_label: String,
    pub chip_order: i32,
    pub value: Option<f64>,
    pub high: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HddTemp {
    pub chip_id: String,
    pub chip_label: String,
    pub sensor_label: String,
    pub chip_order: i32,
    pub value: Option<f64>,
    pub high: Option<f64>,
    pub lowest: Option<f64>,
    pub highest: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Voltage {
    pub chip_id: String,
    pub chip_label: String,
    pub sensor_label: String,
    pub chip_order: i32,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FanSpeed {
    pub chip_id: String,
    pub chip_label: String,
    pub sensor_label: String,
    pub chip_order: i32,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub alarm: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SensorsData {
    pub volts: Vec<Voltage>,
    pub temps: Vec<Temp>,
    pub hdd_temps: Vec<HddTemp>,
    pub fans: Vec<FanSpeed>,
}

enum ChipKind {
    CoreTemp,
    DriveTemp,
    Acpitz,
    Other,
}

fn chip_kind(chip_id: &str) -> ChipKind {
    if chip_id.starts_with("coretemp-") {
        ChipKind::CoreTemp
    } else if chip_id.starts_with("drivetemp-") || chip_id.starts_with("nvme") {
        ChipKind::DriveTemp
    } else if chip_id.starts_with("acpitz-") {
        ChipKind::Acpitz
    } else {
        ChipKind::Other
    }
}

fn get_chip_order(chip_id: &str) -> i32 {
    match chip_kind(chip_id) {
        ChipKind::CoreTemp => 1,
        ChipKind::DriveTemp => 2,
        ChipKind::Acpitz => 3,
        ChipKind::Other => i32::MAX,
    }
}

fn get_custom_chip_label(chip_id: &str, config: &SmConfig) -> String {
    config
        .sensors
        .get(chip_id)
        .and_then(|c| c.get("label"))
        .cloned()
        .unwrap_or_else(|| chip_id.to_string())
}

fn get_custom_sensor_label(chip_id: &str, sensor_id: &str, config: &SmConfig) -> String {
    config
        .sensors
        .get(chip_id)
        .and_then(|c| c.get(sensor_id))
        .cloned()
        .unwrap_or_else(|| sensor_id.to_string())
}

fn is_chip_visible(chip_id: &str, config: &SmConfig) -> bool {
    config
        .sensors
        .get(chip_id)
        .and_then(|c| c.get("visible"))
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(true)
}

fn is_sensor_visible(chip_id: &str, sensor_id: &str, config: &SmConfig) -> bool {
    !config
        .sensors
        .get(chip_id)
        .and_then(|c| c.get("hidden_sensoers"))
        .is_some_and(|hidden| hidden.split(',').any(|h| h == sensor_id))
}

fn parse_sensor_fields<T>(
    fields: &Map<String, Value>,
    make: impl Fn() -> T,
    mut assign: impl FnMut(&mut T, &str, f64),
) -> Option<T> {
    let mut entry: Option<T> = None;
    for (name, value) in fields {
        let Some(v) = value.as_f64() else { continue };
        let e = entry.get_or_insert_with(&make);
        assign(e, name, v);
    }
    entry
}

fn parse_temp_sensor(chip_id: &str, sensor_id: &str, fields: &Map<String, Value>, config: &SmConfig) -> Option<Temp> {
    parse_sensor_fields(
        fields,
        || Temp {
            chip_id: chip_id.to_string(),
            chip_label: get_custom_chip_label(chip_id, config),
            sensor_label: get_custom_sensor_label(chip_id, sensor_id, config),
            chip_order: get_chip_order(chip_id),
            value: None,
            high: None,
        },
        |e, name, v| {
            if name.ends_with("_input") { e.value = Some(v); }
            else if name.ends_with("_max") { e.high = Some(v); }
        },
    )
}

fn parse_hdd_sensor(chip_id: &str, sensor_id: &str, fields: &Map<String, Value>, config: &SmConfig) -> Option<HddTemp> {
    parse_sensor_fields(
        fields,
        || HddTemp {
            chip_id: chip_id.to_string(),
            chip_label: get_custom_chip_label(chip_id, config),
            sensor_label: get_custom_sensor_label(chip_id, sensor_id, config),
            chip_order: get_chip_order(chip_id),
            value: None,
            high: None,
            lowest: None,
            highest: None,
        },
        |e, name, v| {
            if name.ends_with("_input") { e.value = Some(v); }
            else if name.ends_with("_max") && v <= DRIVE_TEMP_MAX_CAP { e.high = Some(v); }
            else if name.ends_with("_lowest") { e.lowest = Some(v); }
            else if name.ends_with("_highest") { e.highest = Some(v); }
        },
    )
}

fn parse_fan_sensor(chip_id: &str, sensor_id: &str, fields: &Map<String, Value>, config: &SmConfig) -> Option<FanSpeed> {
    parse_sensor_fields(
        fields,
        || FanSpeed {
            chip_id: chip_id.to_string(),
            chip_label: get_custom_chip_label(chip_id, config),
            sensor_label: get_custom_sensor_label(chip_id, sensor_id, config),
            chip_order: get_chip_order(chip_id),
            value: None,
            min: None,
            alarm: None,
        },
        |e, name, v| {
            if name.ends_with("_input") { e.value = Some(v); }
            else if name.ends_with("_min") { e.min = Some(v); }
            else if name.ends_with("_alarm") { e.alarm = Some(v != 0.0); }
        },
    )
}

fn parse_volt_sensor(chip_id: &str, sensor_id: &str, fields: &Map<String, Value>, config: &SmConfig) -> Option<Voltage> {
    parse_sensor_fields(
        fields,
        || Voltage {
            chip_id: chip_id.to_string(),
            chip_label: get_custom_chip_label(chip_id, config),
            sensor_label: get_custom_sensor_label(chip_id, sensor_id, config),
            chip_order: get_chip_order(chip_id),
            value: None,
            min: None,
            max: None,
        },
        |e, name, v| {
            if name.ends_with("_input") { e.value = Some(v); }
            else if name.ends_with("_min") { e.min = Some(v); }
            else if name.ends_with("_max") { e.max = Some(v); }
        },
    )
}

fn parse_sensors_json(sensors_json: &Value, config: &SmConfig) -> SensorsData {
    let mut output = SensorsData::default();
    let Value::Object(sensors_json) = sensors_json else { return output };

    for (chip_id, chip_data) in sensors_json {
        if !is_chip_visible(chip_id, config) {
            continue;
        }
        let Value::Object(chip_data) = chip_data else { continue };
        let is_drive = matches!(chip_kind(chip_id), ChipKind::DriveTemp);

        for (sensor_id, sensor_values) in chip_data {
            let Value::Object(fields) = sensor_values else { continue };
            if !is_sensor_visible(chip_id, sensor_id, config) {
                continue;
            }

            if fields.keys().any(|k| k.starts_with("temp")) {
                if is_drive {
                    output.hdd_temps.extend(parse_hdd_sensor(chip_id, sensor_id, fields, config));
                } else {
                    output.temps.extend(parse_temp_sensor(chip_id, sensor_id, fields, config));
                }
            } else if fields.keys().any(|k| k.starts_with("fan")) {
                output.fans.extend(parse_fan_sensor(chip_id, sensor_id, fields, config));
            } else if fields.keys().any(|k| k.starts_with("in")) {
                output.volts.extend(parse_volt_sensor(chip_id, sensor_id, fields, config));
            }
        }
    }

    macro_rules! sort_by_chip {
        ($v:expr) => {
            $v.sort_by(|a, b| a.chip_order.cmp(&b.chip_order).then_with(|| a.chip_id.cmp(&b.chip_id)));
        };
    }
    sort_by_chip!(output.temps);
    sort_by_chip!(output.hdd_temps);
    sort_by_chip!(output.volts);
    sort_by_chip!(output.fans);

    output
}

fn get_sensors_data_from_command(lm_sensors_config: Option<&str>) -> color_eyre::Result<Value> {
    let output = match Command::new("sensors")
        .args(["-c", lm_sensors_config.unwrap_or(NULL_DEVICE), "-j"])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                bail!("The `sensors` command was not found. Please make sure `lm-sensors` is installed and in your PATH.");
            } else {
                return Err(e.into());
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Failed to execute `sensors` command. Exit code: {}. Stderr: {}",
            output.status,
            stderr
        );
    }

    let stdout = String::from_utf8(output.stdout)?;
    let data: Value = serde_json::from_str(&stdout)?;
    Ok(data)
}

fn get_sensors_data_from_file(path: &str) -> color_eyre::Result<Value> {
    let content = std::fs::read_to_string(path)?;
    let data: Value = serde_json::from_str(&content)?;
    Ok(data)
}

pub fn get_data(
    lm_sensors_config: Option<&str>,
    lm_sensors_json: Option<&str>,
    config: &SmConfig,
) -> color_eyre::Result<SensorsData> {
    let raw_sensor_data = if let Some(path) = lm_sensors_json {
        get_sensors_data_from_file(path)?
    } else {
        get_sensors_data_from_command(lm_sensors_config)?
    };

    Ok(parse_sensors_json(&raw_sensor_data, config))
}