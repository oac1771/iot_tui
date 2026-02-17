use std::{
    io,
    sync::mpsc::{self, Sender},
    thread,
};

use crossterm::event::{KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, ListState, StatefulWidget, Widget},
};

pub struct Page {
    exit: bool,
}

enum Event {
    UserInput(crossterm::event::KeyEvent),
}

impl Page {
    pub fn new() -> Self {
        Self { exit: false }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let (events_tx, events_rx) = mpsc::channel::<Event>();

        thread::spawn(move || {
            handle_input_events(events_tx).unwrap();
        });

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            match events_rx.recv().unwrap() {
                Event::UserInput(key_event) => self.handle_key_event(key_event)?,
            }
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
            self.exit = true;
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}

impl Widget for &Page {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)]);
        let [title_area, data_area] = layout.areas(area);

        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);

        let _title_block = Block::bordered()
            .title(Line::from("  Foo overview  ").bold().centered())
            .border_set(border::DOUBLE)
            .title_bottom(instructions.centered())
            .render(title_area, buf);

        let data_block = Block::bordered()
            .title(Line::from("  Data overview  ").bold().centered())
            .border_set(border::DOUBLE);

        let items = ["Item 1", "Item 2", "Item 3"];
        let list_items: Vec<ListItem> = items
            .iter()
            .map(|s| ListItem::new(Line::from(*s).alignment(Alignment::Center)))
            .collect();

        let mut state = ListState::default();
        state.select(Some(0)); // select first item

        let _list = StatefulWidget::render(
            List::new(list_items)
                .block(data_block)
                .highlight_symbol(">> ")
                .highlight_style(Style::new().bold())
                .repeat_highlight_symbol(true),
            data_area,
            buf,
            &mut state,
        );

        // split data block into two
    }
}

fn handle_input_events(events_tx: Sender<Event>) -> io::Result<()> {
    loop {
        match crossterm::event::read().unwrap() {
            crossterm::event::Event::Key(key_event) => {
                events_tx.send(Event::UserInput(key_event)).unwrap()
            }
            _ => {}
        }
    }
}
