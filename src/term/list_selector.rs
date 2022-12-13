use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget, Widget},
};

pub trait ListSelectorAction {
    fn render_style(&self, string: &str, selected: bool, scrolling_on: bool) -> Style;
}

pub struct ListSelector<Action> {
    list: Vec<(String, Action)>,
    current_position: usize,
    scroll_position: usize,
}

impl<Action> Default for ListSelector<Action> {
    fn default() -> Self {
        Self { list: Default::default(), current_position: Default::default(), scroll_position: Default::default() }
    }
}

impl<Action> ListSelector<Action> {
    pub fn get_item_frame(&self, height: usize) -> Vec<(usize, &(String, Action))> {
        let height = height.saturating_sub(2); // Remove the borders
                                               // Add a little offset when the list is full
        let start = self.scroll_position.saturating_sub(3);
        let length = self.list.len();
        let length_after_start = length.saturating_sub(start);
        // Tries to take all the space left if length_after_start is smaller than height
        let start = start - height.saturating_sub(length_after_start);
        self.list
            .iter()
            .enumerate()
            .skip(start)
            .take(height)
            .collect::<Vec<_>>()
    }

    pub fn is_scrolling(&self) -> bool {
        self.scroll_position != self.current_position
    }

    pub fn click_on(&mut self, y_position: usize, height: usize) -> Option<(usize, &Action)> {
        self.get_item_frame(height)
            .iter()
            .enumerate()
            .find(|(i, _)| *i == y_position)
            .map(|(_, w)| (w.0, &w.1 .1))
    }

    pub fn play(&mut self) -> Option<&Action> {
        self.current_position = self.scroll_position;
        self.list
            .get(self.current_position)
            .map(|(_, action)| action)
    }

    pub fn scroll_down(&mut self) {
        self.scroll_to(self.scroll_position.saturating_add(1));
    }

    pub fn scroll_up(&mut self) {
        self.scroll_to(self.scroll_position.saturating_sub(1));
    }

    pub fn scroll_to(&mut self, position: usize) {
        self.scroll_position = position.min(self.list.len() - 1);
    }

    pub fn scroll(&self) -> Option<&Action> {
        self.list
            .get(self.scroll_position)
            .map(|(_, action)| action)
    }

    pub fn select(&self) -> Option<&Action> {
        self.list
            .get(self.current_position)
            .map(|(_, action)| action)
    }

    pub fn select_down(&mut self) {
        self.select_to(self.current_position.saturating_add(1));
    }

    pub fn select_up(&mut self) {
        self.select_to(self.current_position.saturating_sub(1));
    }

    pub fn select_to(&mut self, position: usize) {
        self.current_position = position.min(self.list.len() - 1);
    }

    pub fn update(&mut self, list: Vec<(String, Action)>, current: usize) {
        if !self.is_scrolling() {
            self.scroll_position = current;
        }
        self.current_position = current;
        self.list = list;
        self.current_position = self.current_position.min(self.list.len() - 1);
        self.scroll_position = self.scroll_position.min(self.list.len() - 1);
    }
}

impl<Action: ListSelectorAction> Widget for &ListSelector<Action> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(
            List::new(
                self.get_item_frame(area.height as usize)
                    .iter()
                    .map(|(i, (string, action))| {
                        let style = action.render_style(
                            string,
                            self.current_position == *i,
                            self.scroll_position == *i,
                        );
                        ListItem::new(Text::from(string.as_str())).style(style)
                    })
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title(" Playlist ")),
            area,
            buf,
            &mut ListState::default(),
        );
    }
}
