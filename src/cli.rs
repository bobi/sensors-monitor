use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct SmArgs {
    #[clap(
        short = 'r',
        long = "refresh",
        value_name = "Refresh interval in seconds",
        default_value = Some("2")
    )]
    pub refresh: Option<u16>,
    #[clap(
        short = 'l',
        long = "lm-sensors-config",
        value_name = "lm-sensors config file"
    )]
    pub lm_sensors_config: Option<String>,
    #[clap(
        short = 'j',
        long = "lm-sensors-json",
        value_name = "lm-sensors JSON output file path"
    )]
    pub lm_sensors_json: Option<String>,
    #[clap(
        short = 'c',
        long = "config",
        value_name = "Config file path",
        default_value = Some("/etc/sensors-monitor.conf")
    )]
    pub config: Option<String>,
}
