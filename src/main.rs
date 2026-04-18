use crate::{cli::SmArgs, config::SmConfig};
use clap::Parser;
use color_eyre::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
};
use crossterm::execute;
use ratatui::DefaultTerminal;
use std::{
    io::stdout,
    time::{Duration, Instant},
};

mod cli;
mod config;
mod sensors;
mod ui;

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = SmArgs::parse();
    let config = config::load_config(&args.config)?;

    let mut stdout = stdout();
    execute!(stdout, EnableMouseCapture)?;

    let terminal = ratatui::init();
    let res = App::new(&args, &config).run(terminal);

    ratatui::restore();
    execute!(stdout, DisableMouseCapture)?;

    if let Err(err) = res {
        eprintln!("{:?}", err);
    }

    Ok(())
}

struct App<'a> {
    exit: bool,
    config: &'a SmConfig,
    refresh_rate: Duration,
    lm_sensors_config: Option<String>,
    lm_sensors_json: Option<String>,
}

const TICK_RATE: u64 = 100;

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
        let tick_rate = Duration::from_millis(TICK_RATE);
        let mut last_tick = Instant::now();
        let mut last_refresh = Instant::now();

        let mut sensor_data = sensors::get_data(
            &self.lm_sensors_config,
            &self.lm_sensors_json,
            self.config,
        )?;

        while self.is_running() {
            if last_refresh.elapsed() >= self.refresh_rate {
                sensor_data = sensors::get_data(
                    &self.lm_sensors_config,
                    &self.lm_sensors_json,
                    self.config,
                )?;
                last_refresh = Instant::now();
            }

            terminal.draw(|f| {
                f.render_widget(
                    ui::SmUi::new(&sensor_data, &self.refresh_rate),
                    f.area(),
                )
            })?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? && let Event::Key(key) = event::read()? {
                self.handle_key_press(key);
            }

            if last_tick.elapsed() >= tick_rate {
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