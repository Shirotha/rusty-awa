use std::{fmt::Display, marker::PhantomData};

use awa_core::Abyss;
use ratatui::{prelude::*, widgets::*};

#[derive(Debug, Clone, Copy)]
pub struct AbyssDisplay<A: Abyss + Display> {
    scroll: u16,
    _phantom: PhantomData<A>,
}
impl<A: Abyss + Display> AbyssDisplay<A> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            scroll: 0,
            _phantom: PhantomData,
        }
    }
    #[inline]
    pub fn scroll(&mut self, direction: ScrollDirection) {
        self.scroll = match direction {
            ScrollDirection::Backward => self.scroll.saturating_sub(1),
            ScrollDirection::Forward => self.scroll.saturating_add(1),
        }
    }
}
impl<A: Abyss + Display> Default for AbyssDisplay<A> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
impl<A: Abyss + Display> StatefulWidgetRef for AbyssDisplay<A> {
    type State = A;
    #[inline]
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let str = format!("{}", state);
        let lines = str.lines().collect::<Vec<_>>();
        let layout =
            Layout::horizontal(vec![Constraint::Length(2), Constraint::Fill(1)]).split(area);
        let mut scroll_state = ScrollbarState::new(lines.len())
            .position(self.scroll as usize)
            .content_length(lines.len())
            .viewport_content_length(area.height as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalLeft).render(
            layout[0],
            buf,
            &mut scroll_state,
        );
        Paragraph::new(Text::from_iter(lines))
            .scroll((self.scroll, 0))
            .render(layout[1], buf);
    }
}
