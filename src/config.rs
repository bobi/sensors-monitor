use config::{Config, ConfigError, Environment, File, FileFormat, Source};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct SmConfigDefaults {
    #[serde(default = "default_refresh")]
    pub refresh: u64,
    pub lm_sensors_json: Option<String>,
    pub lm_sensors_config: Option<String>,
}

fn default_refresh() -> u64 {
    2000
}

#[derive(Debug, Clone, Default)]
pub struct SmConfig {
    pub defaults: SmConfigDefaults,
    pub sensors: HashMap<String, HashMap<String, String>>,
}

impl Default for SmConfigDefaults {
    fn default() -> Self {
        Self {
            refresh: default_refresh(),
            lm_sensors_json: None,
            lm_sensors_config: None,
        }
    }
}

const DEFAULTS_SECTION: &str = "defaults";

pub fn load_config(config_file: &str) -> Result<(SmConfig, Vec<String>), ConfigError> {
    let config = Config::builder()
        .add_source(
            File::with_name(config_file)
                .format(FileFormat::Ini)
                .required(false),
        )
        .add_source(Environment::with_prefix("SM_"))
        .build()?;

    let mut sm_config = SmConfig::default();
    let mut warnings: Vec<String> = Vec::new();

    match config.collect() {
        Ok(config_table) => {
            for (key, value) in config_table {
                if key == DEFAULTS_SECTION {
                    match value.try_deserialize::<SmConfigDefaults>() {
                        Ok(defaults) => sm_config.defaults = defaults,
                        Err(e) => warnings.push(format!("Failed to parse [defaults] section: {e}")),
                    }
                } else if let Ok(section_table) = value.into_table() {
                    let mut section = HashMap::new();
                    for (sub_key, sub_value) in section_table {
                        section.insert(sub_key, sub_value.into_string().unwrap_or_default());
                    }
                    sm_config.sensors.insert(key, section);
                }
            }
        }
        Err(e) => warnings.push(format!("Failed to read config: {e}")),
    }

    Ok((sm_config, warnings))
}