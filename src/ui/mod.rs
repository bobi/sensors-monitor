mod format;
mod gauge;
mod render;

use crate::sensors;
use humantime::format_duration;
use ratatui::{
    layout::Constraint::{Fill, Length},
    prelude::*,
    style::Style,
    widgets::{Block, Padding, Paragraph, Widget},
};
use render::{draw_empty_panel, render_chip_label, render_fan_combined_row, render_fan_row, render_hdd_combined_row, render_hdd_row, render_temp_row, render_volt_combined_row, render_volt_row};
use std::time::Duration;

const TABLE_BLOCK_PADDING: Padding = Padding::symmetric(2, 1);

#[derive(Debug, Clone)]
pub struct SmUi<'a> {
    data: &'a sensors::SensorsData,
    refresh_rate: &'a Duration,
}

// ── Widget impl ────────────────────────────────────────────────────────────────

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
    pub fn new(data: &'a sensors::SensorsData, refresh_rate: &'a Duration) -> Self {
        SmUi { data, refresh_rate }
    }

    fn draw_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let time_str = time_format::strftime_local(
            "%Y-%m-%d %H:%M:%S",
            time_format::now().expect("Could not get current time"),
        )
        .expect("Could not format time");

        Widget::render(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(
                        " {}  refresh: {}  ",
                        time_str,
                        format_duration(*self.refresh_rate)
                    ),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled("q: quit", Style::default().fg(Color::DarkGray)),
            ])),
            area,
            buf,
        );
    }

    fn draw_system_temperatures(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let temps = &self.data.temps;
        if temps.is_empty() {
            draw_empty_panel(block, area, buf);
            return;
        }

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        enum Entry<'b> {
            Blank,
            ChipLabel(&'b str),
            Sensor(&'b sensors::Temp),
        }

        let mut entries: Vec<Entry> = vec![];
        let mut last_chip: Option<&str> = None;
        let mut prev_was_sensor = false;
        for temp in temps {
            if last_chip != Some(temp.chip_id.as_str()) {
                if last_chip.is_some() {
                    entries.push(Entry::Blank);
                }
                entries.push(Entry::ChipLabel(&temp.chip_label));
                last_chip = Some(&temp.chip_id);
            } else if prev_was_sensor {
                entries.push(Entry::Blank);
            }
            entries.push(Entry::Sensor(temp));
            prev_was_sensor = true;
        }

        let constraints: Vec<Constraint> = entries.iter().map(|_| Length(1)).collect();
        let row_areas = Layout::vertical(constraints).split(inner);

        for (entry, &row_area) in entries.iter().zip(row_areas.iter()) {
            match entry {
                Entry::Blank => {}
                Entry::ChipLabel(label) => render_chip_label(label, row_area, buf),
                Entry::Sensor(t) => {
                    render_temp_row(&t.sensor_label, &t.value, &t.high, row_area, buf)
                }
            }
        }
    }

    fn draw_hdd_temp_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let hdd_temps = &self.data.hdd_temps;
        if hdd_temps.is_empty() {
            draw_empty_panel(block, area, buf);
            return;
        }

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        enum Entry<'b> {
            Blank,
            ChipLabel(&'b str),
            Sensor(&'b sensors::HddTemp),
            CombinedSensor(&'b sensors::HddTemp),
        }

        let mut entries: Vec<Entry> = vec![];
        let mut last_chip: Option<&str> = None;
        let mut prev_was_sensor = false;
        let mut iter = hdd_temps.iter().peekable();
        while let Some(temp) = iter.next() {
            let new_chip = last_chip != Some(temp.chip_id.as_str());
            if new_chip {
                if last_chip.is_some() {
                    entries.push(Entry::Blank);
                }
                last_chip = Some(&temp.chip_id);
                let next_same_chip = iter.peek().map_or(false, |n| n.chip_id == temp.chip_id);
                if !next_same_chip {
                    entries.push(Entry::CombinedSensor(temp));
                    prev_was_sensor = true;
                    continue;
                }
                entries.push(Entry::ChipLabel(&temp.chip_label));
            } else if prev_was_sensor {
                entries.push(Entry::Blank);
            }
            entries.push(Entry::Sensor(temp));
            prev_was_sensor = true;
        }

        let constraints: Vec<Constraint> = entries.iter().map(|_| Length(1)).collect();
        let row_areas = Layout::vertical(constraints).split(inner);

        for (entry, &row_area) in entries.iter().zip(row_areas.iter()) {
            match entry {
                Entry::Blank => {}
                Entry::ChipLabel(label) => render_chip_label(label, row_area, buf),
                Entry::Sensor(t) => render_hdd_row(
                    &t.sensor_label,
                    &t.value,
                    &t.high,
                    &t.lowest,
                    &t.highest,
                    row_area,
                    buf,
                ),
                Entry::CombinedSensor(t) => render_hdd_combined_row(
                    &t.chip_label,
                    &t.value,
                    &t.high,
                    &t.lowest,
                    &t.highest,
                    row_area,
                    buf,
                ),
            }
        }
    }

    fn draw_fans_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let fans = &self.data.fans;
        if fans.is_empty() {
            draw_empty_panel(block, area, buf);
            return;
        }

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        enum Entry<'b> {
            Blank,
            ChipLabel(&'b str),
            Sensor(&'b sensors::FanSpeed),
            CombinedSensor(&'b sensors::FanSpeed),
        }

        let mut entries: Vec<Entry> = vec![];
        let mut last_chip: Option<&str> = None;
        let mut prev_was_sensor = false;
        let mut iter = fans.iter().peekable();
        while let Some(fan) = iter.next() {
            let new_chip = last_chip != Some(fan.chip_id.as_str());
            if new_chip {
                if last_chip.is_some() {
                    entries.push(Entry::Blank);
                }
                last_chip = Some(&fan.chip_id);
                let next_same_chip = iter.peek().map_or(false, |n| n.chip_id == fan.chip_id);
                if !next_same_chip {
                    entries.push(Entry::CombinedSensor(fan));
                    prev_was_sensor = true;
                    continue;
                }
                entries.push(Entry::ChipLabel(&fan.chip_label));
            } else if prev_was_sensor {
                entries.push(Entry::Blank);
            }
            entries.push(Entry::Sensor(fan));
            prev_was_sensor = true;
        }

        let constraints: Vec<Constraint> = entries.iter().map(|_| Length(1)).collect();
        let row_areas = Layout::vertical(constraints).split(inner);

        for (entry, &row_area) in entries.iter().zip(row_areas.iter()) {
            match entry {
                Entry::Blank => {}
                Entry::ChipLabel(label) => render_chip_label(label, row_area, buf),
                Entry::Sensor(f) => render_fan_row(
                    &f.sensor_label,
                    &f.value,
                    &f.min,
                    &f.alarm,
                    row_area,
                    buf,
                ),
                Entry::CombinedSensor(f) => render_fan_combined_row(
                    &f.chip_label,
                    &f.value,
                    &f.min,
                    &f.alarm,
                    row_area,
                    buf,
                ),
            }
        }
    }

    fn draw_voltage_table(&self, area: Rect, buf: &mut Buffer, block: Block<'_>) {
        let voltages = &self.data.volts;
        if voltages.is_empty() {
            draw_empty_panel(block, area, buf);
            return;
        }

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        enum Entry<'b> {
            Blank,
            ChipLabel(&'b str),
            Sensor(&'b sensors::Voltage),
            CombinedSensor(&'b sensors::Voltage),
        }

        let mut entries: Vec<Entry> = vec![];
        let mut last_chip: Option<&str> = None;
        let mut prev_was_sensor = false;
        let mut iter = voltages.iter().peekable();
        while let Some(volt) = iter.next() {
            let new_chip = last_chip != Some(volt.chip_id.as_str());
            if new_chip {
                if last_chip.is_some() {
                    entries.push(Entry::Blank);
                }
                last_chip = Some(&volt.chip_id);
                let next_same_chip = iter.peek().map_or(false, |n| n.chip_id == volt.chip_id);
                if !next_same_chip {
                    entries.push(Entry::CombinedSensor(volt));
                    prev_was_sensor = true;
                    continue;
                }
                entries.push(Entry::ChipLabel(&volt.chip_label));
            } else if prev_was_sensor {
                entries.push(Entry::Blank);
            }
            entries.push(Entry::Sensor(volt));
            prev_was_sensor = true;
        }

        let constraints: Vec<Constraint> = entries.iter().map(|_| Length(1)).collect();
        let row_areas = Layout::vertical(constraints).split(inner);

        for (entry, &row_area) in entries.iter().zip(row_areas.iter()) {
            match entry {
                Entry::Blank => {}
                Entry::ChipLabel(label) => render_chip_label(label, row_area, buf),
                Entry::Sensor(v) => render_volt_row(
                    &v.sensor_label,
                    &v.value,
                    &v.min,
                    &v.max,
                    row_area,
                    buf,
                ),
                Entry::CombinedSensor(v) => render_volt_combined_row(
                    &v.chip_label,
                    &v.value,
                    &v.min,
                    &v.max,
                    row_area,
                    buf,
                ),
            }
        }
    }
}