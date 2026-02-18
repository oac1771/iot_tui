use std::{
    io,
    sync::mpsc::{self, Sender},
    thread,
};

use super::state::{self, StateClient};
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, ListState, StatefulWidget, Widget},
};

pub struct Page;

enum Event {
    UserInput(KeyEvent),
}

impl Page {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut state_client = state::init();

        let (events_tx, events_rx) = mpsc::channel::<Event>();
        thread::spawn(move || {
            loop {
                if let Err(err) = handle_input_events(&events_tx) {
                    println!("Input Error: {:?}", err);
                }
            }
        });

        while !state_client.read_exit() {
            terminal.draw(|frame| self.draw(frame, &mut state_client))?;

            let result = match events_rx.recv() {
                Ok(Event::UserInput(key_event)) => self
                    .handle_key_event(key_event, &state_client)
                    .map_err(|err| err.to_string()),
                Err(err) => Err(err.to_string()),
            };

            if let Err(_) = result {
                state_client.update_is_error();
            }
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        state_client: &StateClient,
    ) -> Result<(), String> {
        let is_error = state_client.read_is_error();

        if key_event.kind == KeyEventKind::Press && !is_error {
            match key_event.code {
                KeyCode::Char('q') => state_client.update_exit(),
                KeyCode::Up => state_client.update_list_item_index(KeyCode::Up)?,
                KeyCode::Down => state_client.update_list_item_index(KeyCode::Down)?,
                _ => {}
            }
        } else if key_event.kind == KeyEventKind::Press && is_error {
            match key_event.code {
                KeyCode::Esc => state_client.update_is_error(),
                _ => {}
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame, state_client: &mut StateClient) {
        frame.render_stateful_widget(self, frame.area(), state_client);
    }
}

impl StatefulWidget for &Page {
    type State = StateClient;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)]);
        let [title_area, data_area] = layout.areas(area);

        let instructions = Line::from(vec![
            " Down ".into(),
            "<Down>".blue().bold(),
            " Up ".into(),
            "<Up>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);

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

        let index = state.read_list_item_index();
        let mut list_state = ListState::default();
        list_state.select(Some(index)); // select first item

        let _list = StatefulWidget::render(
            List::new(list_items)
                .block(data_block)
                .highlight_symbol(">> ")
                .highlight_style(Style::new().bold())
                .repeat_highlight_symbol(true),
            data_area,
            buf,
            &mut list_state,
        );

        if state.read_is_error() {
            let instructions =
                Line::from(vec![" Exit Error Popup ".into(), "<Esc> ".blue().bold()]);

            let block = Block::bordered()
                .title(Line::from("  Error!  ").bold().centered())
                .title_bottom(instructions.centered());
            let area = popup_area(area, 60, 20);

            ratatui::widgets::Clear::render(ratatui::widgets::Clear, area, buf);
            block.render(area, buf);
        }
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical =
        Layout::vertical([Constraint::Percentage(percent_y)]).flex(ratatui::layout::Flex::Center);
    let horizontal =
        Layout::horizontal([Constraint::Percentage(percent_x)]).flex(ratatui::layout::Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn handle_input_events(events_tx: &Sender<Event>) -> Result<(), String> {
    match event::read() {
        Ok(CrosstermEvent::Key(key_event)) => {
            if let Err(err) = events_tx.send(Event::UserInput(key_event)) {
                return Err(err.to_string());
            }
        }
        Err(err) => return Err(err.to_string()),
        _ => {}
    };

    Ok(())
}
