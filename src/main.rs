use crate::cli::SmArgs;
use clap::Parser;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use std::time::{Duration, Instant};

mod cli;
mod config;
mod sensors;
mod ui;

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = SmArgs::parse();
    let (config, config_warnings) = config::load_config(&args.config)?;

    let terminal = ratatui::init();
    let res = App::new(&args, config, config_warnings).run(terminal);
    ratatui::restore();
    res
}

struct App {
    exit: bool,
    config: config::SmConfig,
    config_warnings: Vec<String>,
    refresh_rate: Duration,
    lm_sensors_config: Option<String>,
    lm_sensors_json: Option<String>,
}

const TICK_RATE: Duration = Duration::from_millis(100);

impl App {
    fn new(args: &SmArgs, config: config::SmConfig, config_warnings: Vec<String>) -> Self {
        let refresh_rate = Duration::from_millis(args.refresh.unwrap_or(config.defaults.refresh));
        let lm_sensors_config = args.lm_sensors_config.clone().or_else(|| config.defaults.lm_sensors_config.clone());
        let lm_sensors_json = args.lm_sensors_json.clone().or_else(|| config.defaults.lm_sensors_json.clone());
        Self {
            exit: false,
            config,
            config_warnings,
            refresh_rate,
            lm_sensors_config,
            lm_sensors_json,
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut last_tick = Instant::now();
        let mut last_refresh = Instant::now() - self.refresh_rate;
        let mut sensor_data = sensors::SensorsData::default();
        let mut last_error: Option<String> = None;

        while self.is_running() {
            if last_refresh.elapsed() >= self.refresh_rate {
                match sensors::get_data(
                    self.lm_sensors_config.as_deref(),
                    self.lm_sensors_json.as_deref(),
                    &self.config,
                ) {
                    Ok(data) => {
                        sensor_data = data;
                        last_error = None;
                    }
                    Err(e) => last_error = Some(format!("{e:#}")),
                }
                last_refresh = Instant::now();
            }

            terminal.draw(|f| {
                let mut widget = ui::SmUi::new(&sensor_data, self.refresh_rate);
                if !self.config_warnings.is_empty() {
                    widget = widget.with_config_warnings(&self.config_warnings);
                }
                if let Some(ref err) = last_error {
                    widget = widget.with_error(err);
                }
                f.render_widget(widget, f.area())
            })?;

            let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? && let Event::Key(key) = event::read()? {
                self.handle_key_press(key);
            }

            if last_tick.elapsed() >= TICK_RATE {
                last_tick = Instant::now();
            }
        }
        Ok(())
    }

    fn handle_key_press(&mut self, key: event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        if let KeyCode::Char('q') = key.code {
            self.exit = true;
        }
    }

    fn is_running(&self) -> bool {
        !self.exit
    }
}