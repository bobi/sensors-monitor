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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::SmArgs::parse();

    let config = config::load_config(&args.config)?;

    color_eyre::install()?;
    let mut stdout = stdout();
    execute!(stdout, EnableMouseCapture)?;

    let terminal = ratatui::init();
    let res = App::new(&args, &config).configure().run(terminal);

    ratatui::restore();
    execute!(stdout, DisableMouseCapture)?;

    if let Err(err) = res {
        eprintln!("{:?}", err)
    }

    Ok(())
}

struct App<'a> {
    exit: bool,
    args: &'a SmArgs,
    config: &'a SmConfig,
    refresh_rate: u16,
    lm_sensors_config: Option<String>,
    lm_sensors_json: Option<String>,
}

const TICK_RATE: u64 = 100;

impl<'a> App<'a> {
    const fn new(args: &'a SmArgs, config: &'a SmConfig) -> Self {
        Self {
            exit: false,
            args,
            config,
            refresh_rate: 0,
            lm_sensors_config: None,
            lm_sensors_json: None,
        }
    }

    fn configure(mut self) -> Self {
        self.refresh_rate = self.args.refresh.unwrap_or(self.config.defaults.refresh);

        self.lm_sensors_config = self.args.lm_sensors_config.clone().or(self
            .config
            .defaults
            .lm_sensors_config
            .clone());

        self.lm_sensors_json = self.args.lm_sensors_json.clone().or(self
            .config
            .defaults
            .lm_sensors_json
            .clone());

        self
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<(), Box<dyn std::error::Error>> {
        let tick_rate = Duration::from_millis(TICK_RATE);
        let refresh_duration = Duration::from_secs(u64::from(self.refresh_rate));

        let mut sensor_data = None;
        let mut last_tick = Instant::now();
        let mut last_refresh = Instant::now();

        while self.is_running() {
            if last_refresh.elapsed() >= refresh_duration || sensor_data.is_none() {
                sensor_data = Some(sensors::get_data(
                    &self.lm_sensors_config,
                    &self.lm_sensors_json,
                    self.config,
                )?);

                last_refresh = Instant::now();
            }

            if sensor_data.is_some() {
                terminal.draw(|f| {
                    f.render_widget(
                        ui::SmUi::new(sensor_data.as_ref().unwrap(), self.refresh_rate),
                        f.area(),
                    )
                })?;
            }

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) => self.handle_key_press(key),
                    _ => (),
                }
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
        match key.code {
            KeyCode::Char('q') => self.quit(),
            _ => {}
        }
    }

    fn is_running(&self) -> bool {
        !self.exit
    }

    fn quit(&mut self) {
        self.exit = true;
    }
}
