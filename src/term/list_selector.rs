use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

#[derive(Default)]
pub struct ListSelector {
    pub list_size: usize,
    current_position: usize,
    scroll_position: usize,
}

impl ListSelector {
    pub fn get_item_frame(&self, height: usize) -> (usize, usize) {
        let height = height.saturating_sub(2); // Remove the borders
                                               // Add a little offset when the list is full
        let start = self.scroll_position.saturating_sub(3);
        let length = self.list_size;
        let length_after_start = length.saturating_sub(start);
        // Tries to take all the space left if length_after_start is smaller than height
        let start = start.saturating_sub(height.saturating_sub(length_after_start));
        (
            start.min(self.list_size),
            (start + height).min(self.list_size),
        )
    }

    pub fn is_scrolling(&self) -> bool {
        self.scroll_position != self.current_position
    }

    pub fn click_on(&mut self, y_position: usize, height: usize) -> Option<usize> {
        let (a, b) = self.get_item_frame(height);
        (a..b)
            .enumerate()
            .find(|(i, _)| *i == y_position)
            .map(|(_, w)| w)
    }

    pub fn play(&mut self) -> Option<usize> {
        self.current_position = self.scroll_position;
        self.select()
    }

    pub fn scroll_down(&mut self) {
        self.scroll_to(self.scroll_position.saturating_add(1));
    }

    pub fn scroll_up(&mut self) {
        self.scroll_to(self.scroll_position.saturating_sub(1));
    }

    pub fn scroll_to(&mut self, position: usize) {
        self.scroll_position = position.min(self.list_size.saturating_sub(1));
    }

    pub fn select(&self) -> Option<usize> {
        if self.current_position < self.list_size {
            Some(self.current_position)
        } else {
            None
        }
    }

    pub fn update(&mut self, list_size: usize, current: usize) {
        if !self.is_scrolling() {
            self.scroll_position = current;
        }
        self.current_position = current;
        self.list_size = list_size;
        self.current_position = self.current_position.min(self.list_size.saturating_sub(1));
        self.scroll_position = self.scroll_position.min(self.list_size.saturating_sub(1));
    }

    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        style_fn: impl Fn(usize, bool, bool) -> (Style, String),
        render_title: &str,
    ) {
        let (a, b) = self.get_item_frame(area.height as usize);
        StatefulWidget::render(
            List::new(
                (a..b)
                    .map(|i| {
                        let (style, text) =
                            style_fn(i, self.current_position == i, self.scroll_position == i);
                        ListItem::new(Text::from(text)).style(style)
                    })
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title(render_title)),
            area,
            buf,
            &mut ListState::default(),
        );
    }
}
