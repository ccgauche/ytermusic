use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};

pub struct VerticalGauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    style: Style,
    gauge_style: Style,
}

impl<'a> Widget for VerticalGauge<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let gauge_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        buf.set_style(gauge_area, self.gauge_style);
        if gauge_area.height < 1 {
            return;
        }

        // compute label value and its position
        // label is put at the center of the gauge_area
        let label = {
            let pct = f64::round(self.ratio * 100.0);
            format!("{pct}%")
        };
        let clamped_label_width = gauge_area.width.min(label.len() as u16);
        let label_col = gauge_area.left() + (gauge_area.width - clamped_label_width) / 2;
        let label_row = gauge_area.top() + gauge_area.height / 2;

        // the gauge will be filled proportionally to the ratio
        let filled_height = f64::from(gauge_area.height) * self.ratio;
        let end = gauge_area.bottom() - filled_height.round() as u16;
        for y in gauge_area.top()..end {
            // empty area
            for x in gauge_area.left()..gauge_area.right() {
                buf.get_mut(x, y).set_symbol(" ").set_fg(Color::Reset);
                // No bg set -> transparent!
            }
        }
        for y in end..gauge_area.bottom() {
            // filled area
            for x in gauge_area.left()..gauge_area.right() {
                buf.get_mut(x, y)
                    .set_symbol("█")
                    .set_fg(self.gauge_style.fg.unwrap_or(Color::Reset));
                // No bg set -> transparent background around block
            }
        }
        for x in label_col..label_col + clamped_label_width {
            let cell = buf.get_mut(x, label_row);
            cell.set_symbol(&label[(x - label_col) as usize..(x - label_col + 1) as usize]);

            if gauge_area.height / 2 > end.saturating_sub(2) {
                // Inside filled area
                cell.set_bg(self.gauge_style.fg.unwrap_or(Color::Reset));
                cell.set_fg(Color::Black);
            } else {
                // Inside empty area
                cell.set_fg(Color::White);
                // No bg set, remains transparent
            }
        }
    }
}

impl<'a> Default for VerticalGauge<'a> {
    fn default() -> VerticalGauge<'a> {
        VerticalGauge {
            block: None,
            ratio: 0.0,
            style: Style::default(),
            gauge_style: Style::default(),
        }
    }
}

impl<'a> VerticalGauge<'a> {
    pub fn block(mut self, block: Block<'a>) -> VerticalGauge<'a> {
        self.block = Some(block);
        self
    }

    /// Sets ratio ([0.0, 1.0]) directly.
    pub fn ratio(mut self, ratio: f64) -> VerticalGauge<'a> {
        assert!(
            (0.0..=1.0).contains(&ratio),
            "Ratio should be between 0 and 1 inclusively."
        );
        self.ratio = ratio;
        self
    }

    pub fn gauge_style(mut self, style: Style) -> VerticalGauge<'a> {
        self.gauge_style = style;
        self
    }
}
