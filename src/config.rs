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

#[derive(Debug, Clone)]
pub struct SmConfig {
    pub defaults: SmConfigDefaults,
    pub sensors: HashMap<String, HashMap<String, String>>,
}

impl Default for SmConfig {
    fn default() -> Self {
        Self {
            defaults: Default::default(),
            sensors: Default::default(),
        }
    }
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

pub fn load_config(config_file: &Option<String>) -> Result<SmConfig, ConfigError> {
    let builder = Config::builder();

    let builder = if let Some(file) = config_file {
        builder.add_source(
            File::with_name(file)
                .format(FileFormat::Ini)
                .required(false),
        )
    } else {
        builder
    };

    let config = builder
        .add_source(Environment::with_prefix("SM_"))
        .build()?;

    let mut sm_config: SmConfig = SmConfig {
        defaults: Default::default(),
        sensors: Default::default(),
    };

    if let Ok(config_table) = config.collect() {
        let mut sections = HashMap::new();
        for (key, value) in config_table {
            if key == DEFAULTS_SECTION {
                if let Ok(defaults) = value.try_deserialize::<SmConfigDefaults>() {
                    sm_config.defaults = defaults;
                }
            } else {
                if let Ok(section_table) = value.into_table() {
                    let mut section = HashMap::new();
                    for (sub_key, sub_value) in section_table {
                        section.insert(sub_key, sub_value.into_string().unwrap_or_default());
                    }
                    sections.insert(key, section);
                }
            }
        }
        sm_config.sensors = sections;
    }

    Ok(sm_config)
}
