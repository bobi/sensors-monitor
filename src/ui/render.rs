use ratatui::{
    layout::Constraint::{Fill, Min},
    prelude::*,
    widgets::{Block, Paragraph, Widget},
};
use super::format::{fmt_rpm, fmt_temp, fmt_volts, temp_color};
use super::gauge::BlockGauge;

const LABEL_COL: u16 = 20;

fn row_cols(area: Rect) -> [Rect; 2] {
    Layout::horizontal([Min(LABEL_COL), Fill(1)])
        .spacing(1)
        .areas(area)
}

fn render_text<'a>(text: impl Into<Text<'a>>, area: Rect, buf: &mut Buffer) {
    Widget::render(Paragraph::new(text), area, buf);
}

pub(super) fn render_chip_label(label: &str, area: Rect, buf: &mut Buffer) {
    render_text(label.fg(Color::White).bold(), area, buf);
}

pub(super) fn render_temp_row(
    label: &str,
    value: &Option<f64>,
    high: &Option<f64>,
    area: Rect,
    buf: &mut Buffer,
) {
    let [label_a, gauge_a] = row_cols(area);

    let temp_val = value.unwrap_or(0.0);
    let high_val = high.unwrap_or(100.0);
    let color = temp_color(temp_val, high_val);

    render_text(format!("  {}", label).fg(Color::LightBlue), label_a, buf);

    let ratio = (temp_val / high_val).clamp(0.0, 1.0);

    let label_spans = if let Some(h) = high {
        Line::from(vec![
            fmt_temp(temp_val).fg(Color::White).bold(),
            format!(" / {}", fmt_temp(*h)).fg(Color::Gray),
        ])
    } else {
        Line::from(fmt_temp(temp_val).fg(Color::White).bold())
    };

    BlockGauge::default()
        .gauge_style(Style::default().fg(color).on_dark_gray())
        .ratio(ratio)
        .label(label_spans)
        .render(gauge_a, buf);
}

pub(super) fn render_hdd_row<'a>(
    label: Span<'a>,
    value: &Option<f64>,
    high: &Option<f64>,
    lowest: &Option<f64>,
    highest: &Option<f64>,
    area: Rect,
    buf: &mut Buffer,
) {
    let [label_a, gauge_a] = row_cols(area);
    render_text(label, label_a, buf);
    render_hdd_gauge(value, high, lowest, highest, gauge_a, buf);
}

fn render_hdd_gauge(
    value: &Option<f64>,
    high: &Option<f64>,
    lowest: &Option<f64>,
    highest: &Option<f64>,
    area: Rect,
    buf: &mut Buffer,
) {
    let temp_val = value.unwrap_or(0.0);
    let high_val = high.unwrap_or(100.0);
    let color = temp_color(temp_val, high_val);
    let ratio = (temp_val / high_val).clamp(0.0, 1.0);

    let mut spans = vec![fmt_temp(temp_val).fg(Color::White).bold()];
    if let Some(h) = high {
        spans.push(format!(" / {}", fmt_temp(*h)).fg(Color::Gray));
    }
    if let (Some(lo), Some(hi)) = (*lowest, *highest) {
        spans.push(format!("  ({} – {})", fmt_temp(lo), fmt_temp(hi)).fg(Color::Gray));
    }

    BlockGauge::default()
        .gauge_style(Style::default().fg(color).on_dark_gray())
        .ratio(ratio)
        .label(Line::from(spans))
        .render(area, buf);
}

pub(super) fn render_fan_row<'a>(
    label: Span<'a>,
    value: &Option<f64>,
    min: &Option<f64>,
    alarm: &Option<bool>,
    area: Rect,
    buf: &mut Buffer,
) {
    let [label_a, value_a] = row_cols(area);
    render_text(label, label_a, buf);
    render_fan_value(value, min, alarm, value_a, buf);
}

fn render_fan_value(
    value: &Option<f64>,
    min: &Option<f64>,
    alarm: &Option<bool>,
    area: Rect,
    buf: &mut Buffer,
) {
    let color = if alarm.unwrap_or(false) { Color::Red } else { Color::LightGreen };
    let mut spans = vec![value.map(fmt_rpm).unwrap_or_default().fg(color).bold()];
    if let Some(m) = min {
        spans.push(format!("  (min: {})", fmt_rpm(*m)).fg(Color::Gray));
    }
    render_text(Line::from(spans), area, buf);
}

pub(super) fn render_volt_row<'a>(
    label: Span<'a>,
    value: &Option<f64>,
    min: &Option<f64>,
    max: &Option<f64>,
    area: Rect,
    buf: &mut Buffer,
) {
    let [label_a, value_a] = row_cols(area);
    render_text(label, label_a, buf);
    render_volt_value(value, min, max, value_a, buf);
}

fn render_volt_value(
    value: &Option<f64>,
    min: &Option<f64>,
    max: &Option<f64>,
    area: Rect,
    buf: &mut Buffer,
) {
    let val = value.unwrap_or(0.0);
    let color = if val < min.unwrap_or(f64::NEG_INFINITY) {
        Color::Yellow
    } else if val > max.unwrap_or(f64::INFINITY) {
        Color::Red
    } else {
        Color::LightGreen
    };
    let mut spans = vec![fmt_volts(val).fg(color).bold()];
    if let (Some(lo), Some(hi)) = (*min, *max) {
        spans.push(format!("  ({} – {})", fmt_volts(lo), fmt_volts(hi)).fg(Color::Gray));
    }
    render_text(Line::from(spans), area, buf);
}

pub(super) fn draw_empty_panel(block: Block<'_>, area: Rect, buf: &mut Buffer) {
    let inner = block.inner(area);
    Widget::render(block, area, buf);
    Widget::render(
        Paragraph::new(Line::from("No data").fg(Color::DarkGray)).alignment(Alignment::Center),
        inner,
        buf,
    );
}