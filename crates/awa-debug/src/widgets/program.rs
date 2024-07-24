use std::collections::HashSet;

use awa_core::Program;
use ratatui::{prelude::*, widgets::*};
use style::Styled;

#[derive(Debug, Clone, Copy)]
pub struct ProgramWindow<'a> {
    program: &'a Program,
    pc: usize,
    scroll: usize,
    line_digits: usize,
}
impl<'a> ProgramWindow<'a> {
    #[inline]
    pub fn new(program: &'a Program) -> Self {
        Self {
            program,
            pc: 0,
            scroll: 0,
            line_digits: (program.len() as f64).log10().trunc() as usize + 1,
        }
    }
    #[inline(always)]
    pub fn min_width(&self) -> usize {
        self.line_digits + 9
    }
    #[inline(always)]
    pub fn set_pc(&mut self, pc: usize) {
        self.pc = pc;
        self.scroll = pc.saturating_sub(5);
    }
    #[inline]
    pub fn scroll(&mut self, direction: ScrollDirection) {
        self.scroll = match direction {
            ScrollDirection::Backward => self.scroll.saturating_sub(1),
            ScrollDirection::Forward => self
                .scroll
                .saturating_add(1)
                .min(self.program.len().saturating_sub(1)),
        };
    }
    const NUMBER_STYLE: Style = Style::new().fg(Color::Gray);
    const BREAKPOINT_STYLE: Style = Style::new().fg(Color::Black).bg(Color::LightRed);
    const AWATISM_STYLE: Style = Style::new().fg(Color::White);
    const CENTER_STYLE: Style = Style::new().fg(Color::Black).bg(Color::White);
}
impl<'a> StatefulWidgetRef for ProgramWindow<'a> {
    type State = HashSet<usize>;
    #[inline]
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Text::from_iter(
            self.program
                .iter()
                .enumerate()
                .skip(self.scroll)
                .take(area.height as usize)
                .map(|(pc, awatism)| {
                    let mut number = (pc + 1).to_string();
                    for _ in number.len()..self.line_digits {
                        number.push(' ')
                    }
                    let number = number.set_style(if state.contains(&pc) {
                        Self::BREAKPOINT_STYLE
                    } else {
                        Self::NUMBER_STYLE
                    });
                    let instruction = awatism.to_string().set_style(if pc == self.pc {
                        Self::CENTER_STYLE
                    } else {
                        Self::AWATISM_STYLE
                    });
                    Line::default().spans(vec![number, " ".into(), instruction])
                }),
        )
        .render(area, buf)
    }
}
