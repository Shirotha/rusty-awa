mod io;
pub use io::*;
mod program;
pub use program::*;
mod abyss;
pub use abyss::*;

use awa_core::{Abyss, Program};
use ratatui::{prelude::*, widgets::*};
use std::{fmt::Display, mem::transmute};

#[derive(Debug)]
pub struct State<'a, 'b, A: Abyss + Display> {
    pub program: &'b mut <ProgramWindow<'a> as StatefulWidgetRef>::State,
    pub abyss: &'b mut <AbyssDisplay<A> as StatefulWidgetRef>::State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
pub enum Tab {
    IO = 0,
    Abyss = 1,
    Diagnostics = 2,
}
impl Tab {
    pub const COUNT: usize = 3;
    #[inline]
    pub fn next(self) -> Self {
        let tab = ((self as usize) + 1) % Self::COUNT;
        // SAFETY: tab is always a valid Tab
        unsafe { transmute(tab) }
    }
    #[inline]
    pub fn prev(self) -> Self {
        let tab = ((self as usize) + (Self::COUNT - 1)) % Self::COUNT;
        // SAFETY: tab is always a valid Tab
        unsafe { transmute(tab) }
    }
}

#[derive(Debug, Clone)]
pub struct View<'a, A: Abyss + Display> {
    pub active_tab: Tab,
    pub scroll_size: usize,
    pub program: ProgramWindow<'a>,
    pub abyss: AbyssDisplay<A>,
    pub io: MirrorIO,
    pub diagnostics: MirrorIO,
}
impl<'a, A: Abyss + Display> View<'a, A> {
    #[inline]
    pub fn new(program: &'a Program, initial_tab: Tab, scroll_size: usize) -> Self {
        Self {
            active_tab: initial_tab,
            scroll_size,
            program: ProgramWindow::new(program),
            abyss: AbyssDisplay::new(),
            io: MirrorIO::new(),
            diagnostics: MirrorIO::new(),
        }
    }
    #[inline]
    pub fn cycle(&mut self, direction: ScrollDirection) {
        self.active_tab = match direction {
            ScrollDirection::Forward => self.active_tab.next(),
            ScrollDirection::Backward => self.active_tab.prev(),
        };
    }
    #[inline]
    pub fn scroll(&mut self, direction: ScrollDirection) {
        match self.active_tab {
            Tab::IO => self.io.scroll(direction),
            Tab::Abyss => self.abyss.scroll(direction),
            Tab::Diagnostics => self.diagnostics.scroll(direction),
        }
    }
    const TAB_STYLE: Style = Style::new();
    const ACTIVE_TAB_STYLE: Style = Style::new().fg(Color::White).bg(Color::DarkGray);
}
impl<'a, A: Abyss + Display + 'a> StatefulWidgetRef for View<'a, A> {
    type State = State<'a, 'a, A>;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let outer = Layout::horizontal(vec![
            Constraint::Length(self.program.min_width() as u16),
            Constraint::Fill(1),
        ])
        .split(area);
        Block::bordered().render(outer[1], buf);
        let inner =
            Layout::vertical(vec![Constraint::Length(1), Constraint::Fill(1)]).split(outer[1]);
        self.program.render_ref(outer[0], buf, state.program);
        Tabs::new(vec!["I/O", "Abyss", "Diagnostics"])
            .style(Self::TAB_STYLE)
            .highlight_style(Self::ACTIVE_TAB_STYLE)
            .divider("-")
            .select(self.active_tab as usize)
            .render(inner[0].inner(Margin::new(2, 0)), buf);
        let mut content = inner[1];
        content.x += 1;
        content.width -= 2;
        content.height -= 1;
        match self.active_tab {
            Tab::IO => self.io.render_ref(content, buf),
            Tab::Abyss => self.abyss.render_ref(content, buf, state.abyss),
            Tab::Diagnostics => self.diagnostics.render_ref(content, buf),
        }
    }
}
