use crate::sensors;
use humantime::format_duration;
use ratatui::{
    layout::Constraint::Fill,
    prelude::*,
    style::Style,
    widgets::{Block, Cell, Padding, Row, Table, Widget},
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SmUi<'a> {
    data: &'a sensors::SensorsData,
    refresh_rate: &'a Duration,
}

fn get_colored_temp(temp: &Option<f64>, high: &Option<f64>) -> Cell<'static> {
    let temp_val = temp.unwrap_or_else(|| 0.0);

    let high_val = high.unwrap_or(f64::MAX);

    let style = if temp_val >= high_val * 0.8 {
        Style::default().fg(Color::Red).bold()
    } else if temp_val >= high_val * 0.6 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::LightGreen).bold()
    };

    cell_current_value(val_temp(temp)).style(style)
}

fn get_colored_voltage(
    voltage: &Option<f64>,
    min: &Option<f64>,
    max: &Option<f64>,
) -> Cell<'static> {
    let voltage_val = voltage.unwrap_or_else(|| 0.0);

    let min_val = min.unwrap_or(f64::MIN);
    let max_val = max.unwrap_or(f64::MAX);

    let style = if voltage_val < min_val {
        Style::default().fg(Color::Yellow).bold()
    } else if voltage_val > max_val {
        Style::default().fg(Color::Red).bold()
    } else {
        Style::default().fg(Color::LightGreen).bold()
    };

    cell_current_value(val_volts(voltage)).style(style)
}

fn fmt_rpm(v: f64) -> String {
    format!("{:.0} RPM", v)
}
fn fmt_temp(v: f64) -> String {
    format!("{:.1}°C", v)
}

fn fmt_volts(v: f64) -> String {
    format!("{:.2}V", v)
}

fn val_volts(val: &Option<f64>) -> String {
    val.map(|v| fmt_volts(v)).unwrap_or_else(|| "".to_string())
}

fn val_rpm(val: &Option<f64>) -> String {
    val.map(|v| fmt_rpm(v)).unwrap_or_else(|| "".to_string())
}

fn val_temp(val: &Option<f64>) -> String {
    val.map(|v| fmt_temp(v)).unwrap_or_else(|| "".to_string())
}

fn header_cell(s: String) -> Cell<'static> {
    Cell::from(Text::from(s).fg(Color::White).left_aligned()).bold()
}

fn cell_value(s: String) -> Cell<'static> {
    Cell::from(Text::from(s).left_aligned()).fg(Color::White)
}
fn cell_current_value(s: String) -> Cell<'static> {
    cell_value(s).fg(Color::LightGreen).bold()
}

fn cell_chip(chip_label: String, sensor_label: String) -> Cell<'static> {
    cell_value(format!("{} {}", chip_label, sensor_label)).fg(Color::LightBlue)
}

fn row_margin_top(last_chip_id: &Option<String>, chip_id: &String) -> u16 {
    match last_chip_id {
        Some(last_chip_id) => {
            if last_chip_id != chip_id {
                1
            } else {
                0
            }
        }
        None => 0,
    }
}

const TABLE_BLOCK_PADDING: Padding = Padding::symmetric(2, 1);
const TABLE_COLUMN_SPACING: u16 = 2;

impl<'a> Widget for SmUi<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_block = Block::default().padding(Padding::symmetric(2, 1));

        let [top_area, bottom_area] = Layout::vertical([Fill(1), Fill(1)])
            .spacing(1)
            .areas(main_block.inner(area));

        let [top_left_area, top_right_area] = Layout::horizontal([Fill(1), Fill(1)])
            .spacing(1)
            .areas(top_area);

        let [bottom_left_area, bottom_right_area] = Layout::horizontal([Fill(1), Fill(1)])
            .spacing(1)
            .areas(bottom_area);

        self.draw_system_temperatures(
            top_left_area,
            buf,
            Block::bordered()
                .padding(TABLE_BLOCK_PADDING)
                .title(Line::from(" System Temperatures ").fg(Color::Cyan).bold()),
        );

        self.draw_fans_table(
            top_right_area,
            buf,
            Block::bordered()
                .padding(TABLE_BLOCK_PADDING)
                .title(Line::from(" Fans ").fg(Color::Cyan).bold())
                .title(
                    Line::from(format!("![{}]!", format_duration(*self.refresh_rate)))
                        .right_aligned(),
                )
                .title(
                    Line::from(format!(
                        "[{}]",
                        time_format::strftime_local(
                            "%Y-%m-%d %H:%M:%S",
                            time_format::now().expect("Could not get current time")
                        )
                        .expect("Could not format time")
                    ))
                    .right_aligned(),
                ),
        );

        self.draw_hdd_temp_table(
            bottom_left_area,
            buf,
            Block::bordered()
                .padding(TABLE_BLOCK_PADDING)
                .title(Line::from(" Drives Temperatures ").fg(Color::Cyan).bold()),
        );

        self.draw_voltage_table(
            bottom_right_area,
            buf,
            Block::bordered()
                .padding(TABLE_BLOCK_PADDING)
                .title(Line::from(" Voltages ").fg(Color::Cyan).bold()),
        );

        main_block.render(area, buf);
    }
}

impl<'a> SmUi<'a> {
    pub fn new(data: &'a sensors::SensorsData, refresh_rate: &'a Duration) -> Self {
        SmUi { data, refresh_rate }
    }

    fn draw_system_temperatures(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let temps = &self.data.temps;
        if temps.is_empty() {
            Widget::render(block, area, buf);
            return;
        }

        let header = Row::new(vec![
            header_cell("Chip / Sensor".to_string()),
            header_cell("Current".to_string()),
            header_cell("High".to_string()).dim(),
            header_cell("Critical".to_string()).dim(),
        ])
        .height(1)
        .bottom_margin(1);

        let mut rows: Vec<Row> = vec![];
        let mut last_chip_id: Option<String> = None;

        for temp in temps {
            let row = Row::new(vec![
                cell_chip(temp.chip_label.clone(), temp.sensor_label.clone()),
                get_colored_temp(&temp.value, &temp.high),
                cell_value(val_temp(&temp.high)).dim(),
                cell_value(val_temp(&temp.critical)).dim(),
            ])
            .top_margin(row_margin_top(&last_chip_id, &temp.chip_id));

            rows.push(row);

            last_chip_id = Some(temp.chip_id.clone());
        }

        let temp_table = Table::new(rows, [Fill(2), Fill(1), Fill(1), Fill(1)])
            .header(header)
            .block(block)
            .column_spacing(TABLE_COLUMN_SPACING);

        Widget::render(temp_table, area, buf);
    }

    fn draw_fans_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let fans = &self.data.fans;
        if fans.is_empty() {
            Widget::render(block, area, buf);
            return;
        }

        let mut rows = vec![];
        let mut last_chip_id: Option<String> = None;

        let header = Row::new(vec![
            header_cell("Fan".to_string()),
            header_cell("Current".to_string()),
            header_cell("Min".to_string()).dim(),
        ])
        .height(1)
        .bottom_margin(1);

        for fan in fans {
            let row = Row::new(vec![
                cell_chip(fan.chip_label.clone(), fan.sensor_label.clone()),
                cell_current_value(val_rpm(&(&fan.value))),
                cell_value(val_rpm(&(&fan.min))).dim(),
            ])
            .top_margin(row_margin_top(&last_chip_id, &fan.chip_id));
            rows.push(row);
            last_chip_id = Some(fan.chip_id.clone());
        }

        let fans_table = Table::new(rows, [Fill(2), Fill(1), Fill(1)])
            .header(header)
            .block(block)
            .column_spacing(TABLE_COLUMN_SPACING);

        Widget::render(fans_table, area, buf);
    }

    fn draw_hdd_temp_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let hdd_temps = &self.data.hdd_temps;
        if hdd_temps.is_empty() {
            Widget::render(block, area, buf);
            return;
        }

        let mut rows = vec![];

        let header = Row::new(vec![
            header_cell("Drive".to_string()),
            header_cell("Current".to_string()),
            header_cell("High".to_string()).dim(),
            header_cell("Critical".to_string()).dim(),
            header_cell("Lowest".to_string()).dim(),
            header_cell("Highest".to_string()).dim(),
        ])
        .height(1)
        .bottom_margin(1);

        for temp in hdd_temps {
            let row = Row::new(vec![
                cell_chip(temp.chip_label.clone(), temp.sensor_label.clone()),
                get_colored_temp(&temp.value, &temp.high),
                cell_value(val_temp(&temp.high)).dim(),
                cell_value(val_temp(&temp.critical)).dim(),
                cell_value(val_temp(&temp.lowest)).dim(),
                cell_value(val_temp(&temp.highest)).dim(),
            ]);
            rows.push(row);
        }

        let hdd_temp_table =
            Table::new(rows, [Fill(3), Fill(1), Fill(1), Fill(1), Fill(1), Fill(1)])
                .header(header)
                .block(block)
                .column_spacing(TABLE_COLUMN_SPACING);

        Widget::render(hdd_temp_table, area, buf);
    }

    fn draw_voltage_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let voltages = &self.data.volts;
        if voltages.is_empty() {
            Widget::render(block, area, buf);
            return;
        }

        let header = Row::new(vec![
            header_cell("Chip / Sensor".to_string()),
            header_cell("Current".to_string()),
            header_cell("Min".to_string()).dim(),
            header_cell("Max".to_string()).dim(),
        ])
        .height(1)
        .bottom_margin(1);

        let mut rows = vec![];
        let mut last_chip_id: Option<String> = None;
        for volt in voltages {
            let row = Row::new(vec![
                cell_chip(volt.chip_label.clone(), volt.sensor_label.clone()),
                get_colored_voltage(&volt.value, &volt.min, &volt.max),
                cell_value(val_volts(&volt.min)).dim(),
                cell_value(val_volts(&volt.max)).dim(),
            ])
            .top_margin(row_margin_top(&last_chip_id, &volt.chip_id));

            rows.push(row);
            last_chip_id = Some(volt.chip_id.clone());
        }

        let voltage_table = Table::new(rows, [Fill(2), Fill(1), Fill(1), Fill(1)])
            .header(header)
            .block(block)
            .column_spacing(TABLE_COLUMN_SPACING);

        Widget::render(voltage_table, area, buf);
    }
}
