use ratatui::{prelude::*, widgets::*};

#[derive(Debug, Clone)]
pub struct MirrorIO {
    lines: Vec<String>,
    scroll: u16,
}
impl MirrorIO {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            lines: Vec::new(),
            scroll: 0,
        }
    }
    #[inline]
    pub fn push(&mut self, str: impl AsRef<str>) {
        let mut line = if self.lines.is_empty() {
            self.lines.push(String::new());
            &mut self.lines[0]
        } else {
            self.lines.last_mut().unwrap()
        };
        for char in str.as_ref().chars() {
            match char {
                '\n' => {
                    self.lines.push(String::new());
                    line = self.lines.last_mut().unwrap();
                }
                char => line.push(char),
            }
        }
        self.scroll = 0;
    }
    #[inline]
    pub fn push_line(&mut self, str: impl AsRef<str>) {
        self.push(str);
        self.lines.push(String::new());
    }
    pub fn scroll(&mut self, direction: ScrollDirection) {
        self.scroll = match direction {
            ScrollDirection::Backward => self.scroll.saturating_sub(1),
            ScrollDirection::Forward => self
                .scroll
                .saturating_add(1)
                .min(self.lines.len().saturating_sub(1) as u16),
        }
    }
}
impl Default for MirrorIO {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
impl WidgetRef for MirrorIO {
    #[inline]
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let layout =
            Layout::horizontal(vec![Constraint::Length(2), Constraint::Fill(1)]).split(area);
        let mut scroll_state = ScrollbarState::new(self.lines.len())
            .position(self.scroll as usize)
            .content_length(self.lines.len())
            .viewport_content_length(area.height as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalLeft).render(
            layout[0],
            buf,
            &mut scroll_state,
        );
        Paragraph::new(Text::from_iter(self.lines.iter().map(String::as_str)))
            .scroll((self.scroll, 0))
            .render(layout[1], buf);
    }
}
