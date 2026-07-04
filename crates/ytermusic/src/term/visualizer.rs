use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{BarChart, Block, Borders, Widget},
};

pub struct Visualizer {
    pub elapsed_ms: u64,
}

impl Visualizer {
    pub fn new(elapsed_ms: u64) -> Self {
        Self { elapsed_ms }
    }
}

impl Widget for Visualizer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = area.width.saturating_sub(2) as usize;
        if width == 0 || area.height < 3 {
            return;
        }

        let mut data = Vec::with_capacity(width / 3);
        let time = (self.elapsed_ms as f64) / 300.0;

        for i in 0..(width / 3) {
            let i_f = i as f64;
            let val = ((time * 2.5 + i_f * 0.5).sin() * 0.5
                + (time * 1.5 - i_f * 0.3).sin() * 0.3
                + (time * 4.0 + i_f * 0.8).cos() * 0.2
                + 1.0)
                / 2.0;

            let height = (val * (area.height.saturating_sub(2)) as f64) as u64;
            data.push(("", height));
        }

        BarChart::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Audio Spectrum "),
            )
            .data(&data)
            .bar_width(2)
            .bar_gap(1)
            .bar_style(Style::default().fg(Color::Rgb(0, 201, 167)))
            .value_style(
                Style::default()
                    .fg(Color::Rgb(13, 15, 26))
                    .bg(Color::Rgb(13, 15, 26)),
            )
            .render(area, buf);
    }
}
