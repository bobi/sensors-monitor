use ratatui::style::Color;

pub(super) fn temp_color(temp_val: f64, high_val: f64) -> Color {
    if temp_val >= high_val * 0.8 {
        Color::Red
    } else if temp_val >= high_val * 0.6 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub(super) fn fmt_temp(v: f64) -> String {
    format!("{:.1}°C", v)
}
pub(super) fn fmt_rpm(v: f64) -> String {
    format!("{:.0} RPM", v)
}
pub(super) fn fmt_volts(v: f64) -> String {
    format!("{:.2}V", v)
}
