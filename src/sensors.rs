use crate::config::SmConfig;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::process::Command;

static NULL_DEVICE: Lazy<String> = Lazy::new(|| "/dev/null".to_string());

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Temp {
    pub chip_id: String,
    pub chip_label: String,
    pub sensor_label: String,
    pub chip_order: i32,
    pub value: Option<f64>,
    pub high: Option<f64>,
    pub critical: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct HddTemp {
    pub chip_id: String,
    pub chip_label: String,
    pub sensor_label: String,
    pub chip_order: i32,
    pub value: Option<f64>,
    pub high: Option<f64>,
    pub critical: Option<f64>,
    pub lowest: Option<f64>,
    pub highest: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
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
#[allow(unused)]
pub struct FanSpeed {
    pub chip_id: String,
    pub chip_label: String,
    pub sensor_label: String,
    pub chip_order: i32,
    pub value: Option<f64>,
    pub min: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SensorsData {
    pub volts: Vec<Voltage>,
    pub temps: Vec<Temp>,
    pub hdd_temps: Vec<HddTemp>,
    pub fans: Vec<FanSpeed>,
}

fn get_chip_order(chip_id: &str) -> i32 {
    let chip_sort_order = [
        (Regex::new("^coretemp-.*").unwrap(), 1),
        (Regex::new("^drivetemp-.*").unwrap(), 2),
        (Regex::new("^acpitz-.*").unwrap(), 3),
    ];

    for (key, value) in &chip_sort_order {
        if key.is_match(chip_id) {
            return *value;
        }
    }

    i32::MAX - 1
}

fn get_custom_chip_label(chip_id: &str, config: &SmConfig) -> String {
    config
        .sensors
        .get(chip_id)
        .and_then(|chip_config| chip_config.get("label"))
        .cloned()
        .unwrap_or_else(|| chip_id.to_string())
}

fn get_custom_sensor_label(chip_id: &str, sensor_id: &str, config: &SmConfig) -> String {
    config
        .sensors
        .get(chip_id)
        .and_then(|chip_config| chip_config.get(sensor_id))
        .cloned()
        .unwrap_or_else(|| sensor_id.to_string())
}

fn is_chip_visible(chip_id: &str, config: &SmConfig) -> bool {
    config
        .sensors
        .get(chip_id)
        .and_then(|chip_config| chip_config.get("visible"))
        .and_then(|visible_str| visible_str.parse::<bool>().ok())
        .unwrap_or(true)
}

fn is_sensor_visible(chip_id: &str, sensor_id: &str, config: &SmConfig) -> bool {
    if let Some(hidden_sensors_str) = config
        .sensors
        .get(chip_id)
        .and_then(|chip_config| chip_config.get("hidden_sensoers"))
    {
        let hidden_sensors: Vec<&str> = hidden_sensors_str.split(',').collect();
        if hidden_sensors.contains(&sensor_id) {
            return false;
        }
    }
    true
}

fn parse_sensors_json(sensors_json: &Value, config: &SmConfig) -> SensorsData {
    let mut output = SensorsData {
        volts: vec![],
        temps: vec![],
        hdd_temps: vec![],
        fans: vec![],
    };

    if let Value::Object(sensors_json) = sensors_json {
        for (chip_id, chip_data) in sensors_json {
            if !is_chip_visible(chip_id, config) {
                continue;
            }
            if let Value::Object(chip_data) = chip_data {
                for (sensor_id, sensor_values) in chip_data {
                    if let Value::Object(sensor_values) = sensor_values {
                        if !is_sensor_visible(chip_id, sensor_id, config) {
                            continue;
                        }

                        let mut temps: HashMap<String, Temp> = HashMap::new();
                        let mut hdd_temps: HashMap<String, HddTemp> = HashMap::new();
                        let mut volts: HashMap<String, Voltage> = HashMap::new();
                        let mut fans: HashMap<String, FanSpeed> = HashMap::new();

                        for (name, value) in sensor_values {
                            let value = value.as_f64().unwrap();
                            if name.starts_with("temp") {
                                if chip_id.starts_with("drivetemp") || chip_id.starts_with("nvme") {
                                    let entry =
                                        hdd_temps.entry(sensor_id.clone()).or_insert(HddTemp {
                                            chip_id: chip_id.clone(),
                                            chip_label: get_custom_chip_label(chip_id, config),
                                            sensor_label: get_custom_sensor_label(
                                                chip_id, sensor_id, config,
                                            ),
                                            chip_order: get_chip_order(chip_id),
                                            value: None,
                                            high: None,
                                            critical: None,
                                            lowest: None,
                                            highest: None,
                                        });
                                    if name.ends_with("_input") {
                                        entry.value = Some(value);
                                    } else if name.ends_with("_max") {
                                        entry.high = Some(value);
                                    } else if name.ends_with("_crit") {
                                        entry.critical = Some(value);
                                    } else if name.ends_with("_lowest") {
                                        entry.lowest = Some(value);
                                    } else if name.ends_with("_highest") {
                                        entry.highest = Some(value);
                                    }
                                } else {
                                    let entry = temps.entry(sensor_id.clone()).or_insert(Temp {
                                        chip_id: chip_id.clone(),
                                        chip_label: get_custom_chip_label(chip_id, config),
                                        sensor_label: get_custom_sensor_label(
                                            chip_id, sensor_id, config,
                                        ),
                                        chip_order: get_chip_order(chip_id),
                                        value: None,
                                        high: None,
                                        critical: None,
                                    });
                                    if name.ends_with("_input") {
                                        entry.value = Some(value);
                                    } else if name.ends_with("_max") {
                                        entry.high = Some(value);
                                    } else if name.ends_with("_crit") {
                                        entry.critical = Some(value);
                                    }
                                }
                            } else if name.starts_with("fan") {
                                let entry = fans.entry(sensor_id.clone()).or_insert(FanSpeed {
                                    chip_id: chip_id.clone(),
                                    chip_label: get_custom_chip_label(chip_id, config),
                                    sensor_label: get_custom_sensor_label(
                                        chip_id, sensor_id, config,
                                    ),
                                    chip_order: get_chip_order(chip_id),
                                    value: None,
                                    min: None,
                                });
                                if name.ends_with("_input") {
                                    entry.value = Some(value);
                                } else if name.ends_with("_min") {
                                    entry.min = Some(value);
                                }
                            } else if name.starts_with("in") {
                                let entry = volts.entry(sensor_id.clone()).or_insert(Voltage {
                                    chip_id: chip_id.clone(),
                                    chip_label: get_custom_chip_label(chip_id, config),
                                    sensor_label: get_custom_sensor_label(
                                        chip_id, sensor_id, config,
                                    ),
                                    chip_order: get_chip_order(chip_id),
                                    value: None,
                                    min: None,
                                    max: None,
                                });
                                if name.ends_with("_input") {
                                    entry.value = Some(value);
                                } else if name.ends_with("_min") {
                                    entry.min = Some(value);
                                } else if name.ends_with("_max") {
                                    entry.max = Some(value);
                                }
                            }
                        }
                        output.temps.extend(temps.into_values());
                        output.hdd_temps.extend(hdd_temps.into_values());
                        output.volts.extend(volts.into_values());
                        output.fans.extend(fans.into_values());
                    }
                }
            }
        }
    }

    output.temps.sort_by_key(|item| item.chip_order);
    output.hdd_temps.sort_by_key(|item| item.chip_order);
    output.volts.sort_by_key(|item| item.chip_order);
    output.fans.sort_by_key(|item| item.chip_order);

    output
}

fn get_sensors_data_from_command(
    lm_sensors_config: &Option<String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let output = match Command::new("sensors")
        .args([
            "-c",
            lm_sensors_config.as_ref().unwrap_or(&NULL_DEVICE),
            "-j",
        ])
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                return Err("The `sensors` command was not found. Please make sure `lm-sensors` is installed and in your PATH.".into());
            } else {
                return Err(e.into());
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to execute `sensors` command. Exit code: {}. Stderr: {}",
            output.status, stderr
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let data: Value = serde_json::from_str(&stdout)?;
    Ok(data)
}

fn get_sensors_data_from_file(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let data: Value = serde_json::from_str(&content)?;
    Ok(data)
}

pub fn get_data(
    lm_sensors_config: &Option<String>,
    lm_sensors_json: &Option<String>,
    config: &SmConfig,
) -> Result<SensorsData, Box<dyn std::error::Error>> {
    let raw_sensor_data = if let Some(path) = lm_sensors_json {
        get_sensors_data_from_file(path)?
    } else {
        get_sensors_data_from_command(lm_sensors_config)?
    };

    let sensor_data = parse_sensors_json(&raw_sensor_data, config);

    Ok(sensor_data)
}