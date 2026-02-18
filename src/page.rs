use std::io;

use super::{
    event::{self, Event},
    state::{self, State, StateClient},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};

pub struct Page;

impl Page {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let state_client = state::init();
        let events_rx = event::init(state_client.clone());

        while state_client.read_exit().is_ok_and(|v| !v) {
            terminal.draw(|frame| self.draw(frame, &state_client).expect("REASON"))?;

            let result = match events_rx.recv() {
                Ok(Event::UserInput(key_event)) => self
                    .handle_key_event(key_event, &state_client)
                    .map_err(|err| err.to_string()),
                Err(err) => Err(err.to_string()),
            };

            if let Err(err) = result {
                state_client.update_error(Some(err)).expect("REASON")
            }
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        state_client: &StateClient,
    ) -> Result<(), String> {
        let is_error = state_client.read_error().map_err(|err| err.to_string())?;

        if key_event.kind == KeyEventKind::Press && is_error.is_none() {
            match key_event.code {
                KeyCode::Esc => state_client.update_exit().map_err(|err| err.to_string())?,
                KeyCode::Up => state_client
                    .update_list_item_index(KeyCode::Up)
                    .map_err(|err| err.to_string())?,
                KeyCode::Down => state_client
                    .update_list_item_index(KeyCode::Down)
                    .map_err(|err| err.to_string())?,
                _ => {}
            }
        } else if key_event.kind == KeyEventKind::Press && is_error.is_some() {
            if let KeyCode::Char('q') = key_event.code {
                state_client
                    .update_error(None)
                    .map_err(|err| err.to_string())?
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame, state_client: &StateClient) -> Result<(), String> {
        let mut state = state_client.to_state().map_err(|err| err.to_string())?;
        frame.render_stateful_widget(self, frame.area(), &mut state);
        Ok(())
    }
}

impl StatefulWidget for &Page {
    type State = State;

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
            "<Esc> ".blue().bold(),
        ]);

        Block::bordered()
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

        let list_index = state.read_list_item_index();
        let mut list_state = ListState::default();
        list_state.select(Some(list_index));

        StatefulWidget::render(
            List::new(list_items)
                .block(data_block)
                .highlight_symbol(">> ")
                .highlight_style(Style::new().bold())
                .repeat_highlight_symbol(true),
            data_area,
            buf,
            &mut list_state,
        );

        if let Some(err) = state.read_error() {
            let instructions = Line::from(vec![" Exit ".into(), "<q> ".blue().bold()]);

            let block = Block::bordered()
                .title(Line::from("  Error!  ").bold().centered())
                .title_bottom(instructions.centered());

            let popup_area = popup_area(area, 80, 80);

            Clear::render(Clear, popup_area, buf);
            Paragraph::new(Line::raw(err))
                .block(block.clone())
                .red()
                .centered()
                .wrap(Wrap { trim: true })
                .render(popup_area, buf);

            block.render(popup_area, buf);
        }
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
