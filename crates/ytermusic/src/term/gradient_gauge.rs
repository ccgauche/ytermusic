use crate::utils::interpolate_color;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{Block, Widget},
};

pub struct GradientGauge<'a> {
    block: Option<Block<'a>>,
    ratio: f64,
    start_color: Color,
    end_color: Color,
    label: String,
}

impl<'a> Widget for GradientGauge<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let gauge_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        if gauge_area.height < 1 || gauge_area.width < 1 {
            return;
        }

        let filled_width = (f64::from(gauge_area.width) * self.ratio).round() as u16;
        let end = gauge_area.left() + filled_width;

        // Render filled area with horizontal gradient using block characters
        for x in gauge_area.left()..end {
            let t = (x - gauge_area.left()) as f64 / gauge_area.width as f64;
            let color = interpolate_color(self.start_color, self.end_color, t);

            for y in gauge_area.top()..gauge_area.bottom() {
                buf.get_mut(x, y).set_symbol("█").set_fg(color);
            }
        }

        // Label
        let label = self.label;
        let clamped_label_width = gauge_area.width.min(label.len() as u16);
        let label_col = gauge_area.left() + (gauge_area.width - clamped_label_width) / 2;
        let label_row = gauge_area.top() + gauge_area.height / 2;

        for x in label_col..label_col + clamped_label_width {
            let char = &label[(x - label_col) as usize..(x - label_col + 1) as usize];
            let cell = buf.get_mut(x, label_row);
            cell.set_symbol(char);

            if x < end {
                let t = (x - gauge_area.left()) as f64 / gauge_area.width as f64;
                let bg_color = interpolate_color(self.start_color, self.end_color, t);
                cell.set_fg(crate::utils::color_contrast(bg_color));
                cell.set_bg(bg_color);
            } else {
                cell.set_fg(Color::White);
                // BG left unchanged to keep global gradient
            }
        }
    }
}

impl<'a> Default for GradientGauge<'a> {
    fn default() -> GradientGauge<'a> {
        GradientGauge {
            block: None,
            ratio: 0.0,
            start_color: Color::Rgb(0, 201, 167),
            end_color: Color::Rgb(99, 102, 241),
            label: String::new(),
        }
    }
}

impl<'a> GradientGauge<'a> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
    pub fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = ratio.clamp(0.0, 1.0);
        self
    }
    pub fn label(mut self, label: String) -> Self {
        self.label = label;
        self
    }
    pub fn gradient(mut self, start: Color, end: Color) -> Self {
        self.start_color = start;
        self.end_color = end;
        self
    }
}
