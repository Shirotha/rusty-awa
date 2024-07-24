#![feature(ptr_as_ref_unchecked)]
use std::{
    collections::HashSet,
    fmt::Display,
    io::{stdout, BufReader, Error as IOError, Read, Write},
    num::ParseIntError,
};

use awa_core::{Abyss, AwaTism, Program};
use awa_interpreter::{Cursor, Error as RuntimeError, Interpreter};

use ratatui::{
    crossterm::{event::*, terminal::*, *},
    prelude::*,
    widgets::*,
};
use thiserror::Error;
use tui_input::{backend::crossterm::EventHandler, Input};

mod pipe;
pub mod widgets;
pub use pipe::*;
use widgets::{State, Tab, View};

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown command")]
    UnknownCommand,
    #[error("line not found in program")]
    InvalidBreakpoint,
    #[error(transparent)]
    RuntimeError(#[from] RuntimeError),
    #[error(transparent)]
    IOError(#[from] IOError),
    #[error(transparent)]
    ParseError(#[from] ParseIntError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    Command,
    Input,
    Done,
    Close,
}

#[derive(Debug)]
pub struct Debugger<'a, A: Abyss + Display> {
    cursor: Cursor<'a>,
    interpreter: Interpreter<A, BufReader<PipeReader>, PipeWriter>,
    inbuffer: Pipe,
    outbuffer: Pipe,
    cmdbuffer: Input,
    breakpoints: HashSet<usize>,
    view: View<'a, A>,
    mode: Mode,
}
impl<'a, A: Abyss + Display + 'a> Debugger<'a, A> {
    #[inline]
    pub fn new(program: &'a Program, abyss: A) -> Self {
        let (inbuffer, outbuffer) = (Pipe::new(), Pipe::new());
        let interpreter =
            Interpreter::new(abyss, BufReader::new(inbuffer.reader()), outbuffer.writer());
        Self {
            cursor: Cursor::new(program),
            interpreter,
            inbuffer,
            outbuffer,
            cmdbuffer: Input::default(),
            breakpoints: HashSet::new(),
            view: View::new(program, Tab::IO, 1),
            mode: Mode::Command,
        }
    }
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next(&mut self) -> Result<(), Error> {
        match self.mode {
            Mode::Command
                if matches!(
                    self.cursor.current(),
                    Some((_, AwaTism::Read)) | Some((_, AwaTism::ReadNum))
                ) =>
            {
                self.view.active_tab = Tab::IO;
                self.mode = Mode::Input;
            }
            Mode::Command | Mode::Input => {
                if !self.cursor.next(&mut self.interpreter)? {
                    self.mode = Mode::Done;
                    return Ok(());
                }
                if let Some(pc) = self.cursor.pc {
                    self.view.program.set_pc(pc);
                    let mut buffer = String::new();
                    // SAFETY: unwrap: reading from Pipe cannot fail
                    self.outbuffer.reader().read_to_string(&mut buffer).unwrap();
                    if !buffer.is_empty() {
                        self.view.io.push(&buffer);
                        self.view.active_tab = Tab::IO;
                    }
                    self.mode = Mode::Command;
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    pub fn run(&mut self) -> Result<(), Error> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;
        while self.mode != Mode::Close {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_event(read()?)?;
        }
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }
    /// # Safety
    /// The result has interier mutable access to `self`.
    pub unsafe fn state<'b>(&self) -> State<'a, 'b, A> {
        let program = (&self.breakpoints as *const _ as *mut HashSet<usize>).as_mut_unchecked();
        let abyss = (self.interpreter.abyss() as *const A as *mut A).as_mut_unchecked();
        State { program, abyss }
    }
    pub fn draw(&mut self, frame: &mut Frame) {
        let outer =
            Layout::vertical(vec![Constraint::Fill(1), Constraint::Length(3)]).split(frame.size());
        // SAFETY: self is not modified before state is dropped
        let mut state = unsafe { self.state() };
        self.view
            .render_ref(outer[0], frame.buffer_mut(), &mut state);
        let title = match self.mode {
            Mode::Command => "Command",
            Mode::Input => "Input",
            _ => return,
        };
        Paragraph::new(Line::from(vec![
            " ".into(),
            self.cmdbuffer.value().into(),
            "|".rapid_blink(),
        ]))
        .block(Block::bordered().title(title))
        .render(outer[1], frame.buffer_mut());
    }
    pub fn handle_event(&mut self, event: Event) -> Result<(), Error> {
        if let Event::Key(
            event @ KeyEvent {
                code,
                modifiers,
                kind,
                ..
            },
        ) = event
        {
            if kind != KeyEventKind::Press {
                return Ok(());
            }
            match code {
                KeyCode::Enter => match self.mode {
                    Mode::Command => {
                        if let Err(error) = self.execute() {
                            self.view.diagnostics.push_line(error.to_string());
                            self.cmdbuffer.reset();
                            self.view.active_tab = Tab::Diagnostics;
                        }
                    }
                    Mode::Input => {
                        // SAFETY: unwrap: writing to Pipe cannot fail
                        self.inbuffer
                            .writer()
                            .write_all(self.cmdbuffer.value().as_bytes())
                            .unwrap();
                        self.view.io.push_line(self.cmdbuffer.value());
                        self.cmdbuffer.reset();
                        self.next()?;
                    }
                    Mode::Done => self.mode = Mode::Close,
                    _ => unreachable!(),
                },
                KeyCode::Tab => self.view.cycle(ScrollDirection::Forward),
                KeyCode::BackTab => self.view.cycle(ScrollDirection::Backward),
                KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.view.scroll(ScrollDirection::Forward)
                }
                KeyCode::Char('k') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.view.scroll(ScrollDirection::Backward)
                }
                KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.view.program.scroll(ScrollDirection::Forward)
                }
                KeyCode::Char('h') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.view.program.scroll(ScrollDirection::Backward)
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.mode = Mode::Close
                }
                _ if matches!(self.mode, Mode::Command | Mode::Input) => {
                    _ = self.cmdbuffer.handle_event(&Event::Key(event))
                }
                _ => (),
            }
        }
        Ok(())
    }
    pub fn execute(&mut self) -> Result<(), Error> {
        fn should_break(this: &mut Debugger<impl Abyss + Display>) -> bool {
            if this.mode != Mode::Command {
                return true;
            }
            if let Some(pc) = this.cursor.pc {
                this.breakpoints.contains(&pc)
            } else {
                this.mode = Mode::Done;
                true
            }
        }
        let cmd = self.cmdbuffer.value();
        let len = cmd.len();
        if len == 0 {
            return self.next();
        }
        // SAFETY: unwrap: cmd is not empty here
        match cmd.chars().next().unwrap() {
            's' if len == 1 => self.next()?,
            's' => {
                let count = cmd[1..].trim().parse::<usize>()?;
                for _ in 0..count {
                    self.next()?;
                    if should_break(self) {
                        break;
                    }
                }
            }
            'r' if len == 1 => loop {
                self.next()?;
                if should_break(self) {
                    break;
                }
            },
            'b' if len == 1 => {
                // SAFETY: unwrap: pc should always be valid by construction
                let pc = self.cursor.pc.unwrap();
                if !self.breakpoints.remove(&pc) {
                    self.breakpoints.insert(pc);
                }
            }
            'b' => {
                let trimmed = cmd[1..].trim();
                if trimmed.starts_with('+') || trimmed.starts_with('-') {
                    let offset = trimmed.parse::<isize>()?;
                    // SAFETY: unwrap: pc should always be valid by construction
                    let pc = (self.cursor.pc.unwrap() as isize + offset) as usize;
                    if pc >= self.cursor.len() {
                        return Err(Error::InvalidBreakpoint);
                    }
                    if !self.breakpoints.remove(&pc) {
                        self.breakpoints.insert(pc);
                    }
                } else {
                    let Some(pc) = trimmed.parse::<usize>()?.checked_sub(1) else {
                        return Err(Error::InvalidBreakpoint);
                    };
                    if pc >= self.cursor.len() {
                        return Err(Error::InvalidBreakpoint);
                    }
                    if !self.breakpoints.remove(&pc) {
                        self.breakpoints.insert(pc);
                    }
                }
            }
            'q' if len == 1 => self.mode = Mode::Close,
            _ => return Err(Error::UnknownCommand),
        };
        self.cmdbuffer.reset();
        Ok(())
    }
}
