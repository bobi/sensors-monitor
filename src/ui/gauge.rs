use ratatui::{prelude::*, symbols, widgets::Block};

#[allow(clippy::struct_field_names)]
#[derive(Debug, Default, Clone)]
pub struct BlockGauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    label: Option<Line<'a>>,
    use_unicode: bool,
    style: Style,
    gauge_style: Style,
}

impl<'a> BlockGauge<'a> {
    pub fn ratio(mut self, ratio: f64) -> Self {
        assert!((0.0..=1.0).contains(&ratio), "Ratio should be between 0 and 1 inclusively.");
        self.ratio = ratio;
        self
    }

    pub fn label(mut self, label: impl Into<Line<'a>>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn gauge_style<S: Into<Style>>(mut self, style: S) -> Self {
        self.gauge_style = style.into();
        self
    }

}

impl Widget for BlockGauge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        self.block.as_ref().render(area, buf);
        let inner = self.block.as_ref().map_or(area, |b| b.inner(area));
        self.render_gauge(inner, buf);
    }
}

impl BlockGauge<'_> {
    fn render_gauge(&self, gauge_area: Rect, buf: &mut Buffer) {
        if gauge_area.is_empty() {
            return;
        }
        buf.set_style(gauge_area, self.gauge_style);

        let default_label = Line::from(format!("{}%", f64::round(self.ratio * 100.0)));
        let label = self.label.as_ref().unwrap_or(&default_label);
        let clamped_label_width = gauge_area.width.min(label.width() as u16);
        let label_col = gauge_area.left() + (gauge_area.width - clamped_label_width) / 2;
        let label_row = gauge_area.top() + gauge_area.height / 2;

        let filled_width = f64::from(gauge_area.width) * self.ratio;
        let end = if self.use_unicode {
            gauge_area.left() + filled_width.floor() as u16
        } else {
            gauge_area.left() + filled_width.round() as u16
        };

        for y in gauge_area.top()..gauge_area.bottom() {
            for x in gauge_area.left()..end {
                let in_label =
                    y == label_row && x >= label_col && x < label_col + clamped_label_width;
                if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
                    if in_label {
                        // Invert fg/bg so the label is readable on the filled bar
                        cell.set_symbol(" ")
                            .set_fg(self.gauge_style.bg.unwrap_or(Color::Reset))
                            .set_bg(self.gauge_style.fg.unwrap_or(Color::Reset));
                    } else {
                        cell.set_symbol(symbols::block::FULL)
                            .set_fg(self.gauge_style.fg.unwrap_or(Color::Reset))
                            .set_bg(self.gauge_style.bg.unwrap_or(Color::Reset));
                    }
                }
            }
            if self.use_unicode && self.ratio < 1.0 {
                if let Some(cell) = buf.cell_mut(Position::new(end, y)) {
                    cell.set_symbol(unicode_block(filled_width % 1.0));
                }
            }
        }

        buf.set_line(label_col, label_row, label, clamped_label_width);
    }
}

fn unicode_block(frac: f64) -> &'static str {
    match (frac * 8.0).round() as u16 {
        1 => symbols::block::ONE_EIGHTH,
        2 => symbols::block::ONE_QUARTER,
        3 => symbols::block::THREE_EIGHTHS,
        4 => symbols::block::HALF,
        5 => symbols::block::FIVE_EIGHTHS,
        6 => symbols::block::THREE_QUARTERS,
        7 => symbols::block::SEVEN_EIGHTHS,
        8 => symbols::block::FULL,
        _ => " ",
    }
}