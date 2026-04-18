mod format;
mod gauge;
mod render;

use crate::sensors;
use humantime::format_duration;
use ratatui::{
    layout::Constraint::{Fill, Length},
    prelude::*,
    widgets::{Block, Padding, Paragraph, Widget},
};
use render::{draw_empty_panel, render_chip_label, render_fan_row, render_hdd_row, render_temp_row, render_volt_row};
use std::time::Duration;

const TABLE_BLOCK_PADDING: Padding = Padding::symmetric(2, 1);

trait SensorItem {
    fn chip_id(&self) -> &str;
    fn chip_label(&self) -> &str;
}

impl SensorItem for sensors::Temp {
    fn chip_id(&self) -> &str { &self.chip_id }
    fn chip_label(&self) -> &str { &self.chip_label }
}

impl SensorItem for sensors::HddTemp {
    fn chip_id(&self) -> &str { &self.chip_id }
    fn chip_label(&self) -> &str { &self.chip_label }
}

impl SensorItem for sensors::FanSpeed {
    fn chip_id(&self) -> &str { &self.chip_id }
    fn chip_label(&self) -> &str { &self.chip_label }
}

impl SensorItem for sensors::Voltage {
    fn chip_id(&self) -> &str { &self.chip_id }
    fn chip_label(&self) -> &str { &self.chip_label }
}

enum SensorEntry<'a, T> {
    Blank,
    ChipLabel(&'a str),
    Sensor(&'a T),
    Combined(&'a T),
}

fn build_sensor_entries<'a, T: SensorItem>(items: &'a [T]) -> Vec<SensorEntry<'a, T>> {
    let mut entries: Vec<SensorEntry<'a, T>> = vec![];
    let mut last_chip: Option<&'a str> = None;
    let mut iter = items.iter().peekable();
    while let Some(item) = iter.next() {
        let new_chip = last_chip != Some(item.chip_id());
        if new_chip {
            if last_chip.is_some() {
                entries.push(SensorEntry::Blank);
            }
            last_chip = Some(item.chip_id());
            let next_same_chip = iter.peek().is_some_and(|n| n.chip_id() == item.chip_id());
            if !next_same_chip {
                entries.push(SensorEntry::Combined(item));
                continue;
            }
            entries.push(SensorEntry::ChipLabel(item.chip_label()));
        } else {
            entries.push(SensorEntry::Blank);
        }
        entries.push(SensorEntry::Sensor(item));
    }
    entries
}

fn render_sensor_panel<T: SensorItem>(
    items: &[T],
    area: Rect,
    buf: &mut Buffer,
    block: Block<'_>,
    render_sensor: impl Fn(&T, Rect, &mut Buffer),
    render_combined: impl Fn(&T, Rect, &mut Buffer),
) {
    if items.is_empty() {
        draw_empty_panel(block, area, buf);
        return;
    }
    let inner = block.inner(area);
    Widget::render(block, area, buf);
    let entries = build_sensor_entries(items);
    let constraints: Vec<Constraint> = entries.iter().map(|_| Length(1)).collect();
    let row_areas = Layout::vertical(constraints).split(inner);
    for (entry, &row_area) in entries.iter().zip(row_areas.iter()) {
        match entry {
            SensorEntry::Blank => {}
            SensorEntry::ChipLabel(label) => render_chip_label(label, row_area, buf),
            SensorEntry::Sensor(t) => render_sensor(t, row_area, buf),
            SensorEntry::Combined(t) => render_combined(t, row_area, buf),
        }
    }
}

// ── Widget impl ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SmUi<'a> {
    data: &'a sensors::SensorsData,
    refresh_rate: Duration,
    error: Option<&'a str>,
}

impl<'a> Widget for SmUi<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_block = Block::default().padding(Padding::symmetric(2, 1));

        let [top_area, bottom_area, status_area] = Layout::vertical([Fill(1), Fill(1), Length(1)])
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
                .title(Line::from(" Fans ").fg(Color::Cyan).bold()),
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

        self.draw_status_bar(status_area, buf);

        main_block.render(area, buf);
    }
}

// ── SmUi methods ───────────────────────────────────────────────────────────────

impl<'a> SmUi<'a> {
    pub fn new(data: &'a sensors::SensorsData, refresh_rate: Duration) -> Self {
        SmUi { data, refresh_rate, error: None }
    }

    pub fn with_error(mut self, error: &'a str) -> Self {
        self.error = Some(error);
        self
    }

    fn draw_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let time_str = time_format::strftime_local(
            "%Y-%m-%d %H:%M:%S",
            time_format::now().expect("Could not get current time"),
        )
        .expect("Could not format time");

        let mut spans = vec![
            format!(" {}  refresh: {}  ", time_str, format_duration(self.refresh_rate)).fg(Color::Gray),
            "q: quit".fg(Color::DarkGray),
        ];
        if let Some(err) = self.error {
            spans.push(format!("  Error: {err}").fg(Color::Red).bold());
        }

        Widget::render(Paragraph::new(Line::from(spans)), area, buf);
    }

    fn draw_system_temperatures(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        render_sensor_panel(
            &self.data.temps,
            area,
            buf,
            block,
            |t, a, b| render_temp_row(format!("  {}", t.sensor_label).fg(Color::LightBlue), &t.value, &t.high, a, b),
            |t, a, b| render_temp_row(t.chip_label.as_str().fg(Color::White).bold(), &t.value, &t.high, a, b),
        );
    }

    fn draw_hdd_temp_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        render_sensor_panel(
            &self.data.hdd_temps,
            area,
            buf,
            block,
            |t, a, b| render_hdd_row(format!("  {}", t.sensor_label).fg(Color::LightBlue), &t.value, &t.high, &t.lowest, &t.highest, a, b),
            |t, a, b| render_hdd_row(t.chip_label.as_str().fg(Color::White).bold(), &t.value, &t.high, &t.lowest, &t.highest, a, b),
        );
    }

    fn draw_fans_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        render_sensor_panel(
            &self.data.fans,
            area,
            buf,
            block,
            |f, a, b| render_fan_row(format!("  {}", f.sensor_label).fg(Color::LightBlue), &f.value, &f.min, &f.alarm, a, b),
            |f, a, b| render_fan_row(f.chip_label.as_str().fg(Color::White).bold(), &f.value, &f.min, &f.alarm, a, b),
        );
    }

    fn draw_voltage_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        render_sensor_panel(
            &self.data.volts,
            area,
            buf,
            block,
            |v, a, b| render_volt_row(format!("  {}", v.sensor_label).fg(Color::LightBlue), &v.value, &v.min, &v.max, a, b),
            |v, a, b| render_volt_row(v.chip_label.as_str().fg(Color::White).bold(), &v.value, &v.min, &v.max, a, b),
        );
    }
}