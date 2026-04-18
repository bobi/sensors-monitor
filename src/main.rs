use crate::{cli::SmArgs, config::SmConfig};
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
    let config = config::load_config(&args.config)?;

    let terminal = ratatui::init();
    let res = App::new(&args, &config).run(terminal);
    ratatui::restore();
    res
}

struct App<'a> {
    exit: bool,
    config: &'a SmConfig,
    refresh_rate: Duration,
    lm_sensors_config: Option<String>,
    lm_sensors_json: Option<String>,
}

const TICK_RATE: Duration = Duration::from_millis(100);

impl<'a> App<'a> {
    fn new(args: &'a SmArgs, config: &'a SmConfig) -> Self {
        Self {
            exit: false,
            config,
            refresh_rate: Duration::from_millis(args.refresh.unwrap_or(config.defaults.refresh)),
            lm_sensors_config: args
                .lm_sensors_config
                .clone()
                .or_else(|| config.defaults.lm_sensors_config.clone()),
            lm_sensors_json: args
                .lm_sensors_json
                .clone()
                .or_else(|| config.defaults.lm_sensors_json.clone()),
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut last_tick = Instant::now();
        let mut last_refresh = Instant::now() - self.refresh_rate;
        let mut sensor_data = sensors::SensorsData::default();

        while self.is_running() {
            if last_refresh.elapsed() >= self.refresh_rate {
                sensor_data = sensors::get_data(
                    self.lm_sensors_config.as_deref(),
                    self.lm_sensors_json.as_deref(),
                    self.config,
                )?;
                last_refresh = Instant::now();
            }

            terminal.draw(|f| {
                f.render_widget(ui::SmUi::new(&sensor_data, self.refresh_rate), f.area())
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
            self.quit();
        }
    }

    fn is_running(&self) -> bool {
        !self.exit
    }

    fn quit(&mut self) {
        self.exit = true;
    }
}