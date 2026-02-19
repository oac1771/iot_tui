use super::{
    event::{self, Event},
    state::{self, State, StateClient},
};
use crate::commands::scan::ScanCmd;
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
use std::io;

pub struct Page;

impl Page {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let state_client = state::init();
        let events_rx = event::init(state_client.clone());

        while state_client.get_exit().is_ok_and(|v| !v) {
            terminal.draw(|frame| {
                self.draw(frame, &state_client)
                    .expect("Failure rendering terminal")
            })?;

            let result = match events_rx.recv() {
                Ok(Event::UserInput(key_event)) => self
                    .handle_key_event(key_event, &state_client)
                    .map_err(|err| err.to_string()),
                Err(err) => Err(err.to_string()),
            };

            if let Err(err) = result {
                state_client
                    .update_error(Some(err))
                    .expect("Failure to update state with error")
            }
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        state_client: &StateClient,
    ) -> Result<(), String> {
        let is_error = state_client.get_error().map_err(|err| err.to_string())?;

        if key_event.kind == KeyEventKind::Press && is_error.is_none() {
            match key_event.code {
                KeyCode::Char('s') => ScanCmd::handle(state_client)?,
                KeyCode::Esc => state_client.update_exit().map_err(|err| err.to_string())?,
                KeyCode::Up => state_client
                    .update_scan_items_index(-1)
                    .map_err(|err| err.to_string())?,
                KeyCode::Down => state_client
                    .update_scan_items_index(1)
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

    fn render_title_area(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(35), Constraint::Percentage(65)]);

        let [cmd_area, meta_data_area] = layout.areas(area);

        let instructions = Line::from(vec![
            " Down ".into(),
            "<Down>".blue().bold(),
            " Up ".into(),
            "<Up>".blue().bold(),
            " Quit ".into(),
            "<Esc> ".blue().bold(),
        ]);

        let cmd_block = Block::bordered()
            .title(Line::from("  ****  ").bold().centered())
            .border_set(border::DOUBLE)
            .title_bottom(instructions.centered());

        Block::bordered()
            .title(Line::from("  ****  ").bold().centered())
            .border_set(border::DOUBLE)
            .render(meta_data_area, buf);

        let cmds = Line::from(vec![" Scan ".into(), "<s>".blue().bold()]);

        Paragraph::new(cmds)
            .block(cmd_block)
            .white()
            .left_aligned()
            .wrap(Wrap { trim: true })
            .render(cmd_area, buf);
    }

    fn render_data_area(&self, area: Rect, buf: &mut Buffer, state: &mut State) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)]);

        let [list_area, data_area] = layout.areas(area);

        Block::bordered()
            .title(Line::from("  ~~~~~~~~~~~~~  ").bold().centered())
            .border_set(border::DOUBLE)
            .render(data_area, buf);

        let list_block = Block::bordered()
            .title(Line::from(" ***** ").bold().centered())
            .border_set(border::DOUBLE);

        let (scan_items_index, scan_items) = state.get_scan_items();

        let scan_list: Vec<ListItem> = scan_items
            .map(|s| ListItem::new(Line::from(s).alignment(Alignment::Left)))
            .collect();

        let mut scan_list_state = ListState::default();
        scan_list_state.select(Some(scan_items_index));

        StatefulWidget::render(
            List::new(scan_list)
                .block(list_block)
                .highlight_symbol(">> ")
                .highlight_style(Style::new().bold())
                .repeat_highlight_symbol(true),
            list_area,
            buf,
            &mut scan_list_state,
        );
    }

    fn render_error_popup(&self, area: Rect, buf: &mut Buffer, err: &str) {
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

impl StatefulWidget for &Page {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)]);
        let [title_area, data_area] = layout.areas(area);

        self.render_title_area(title_area, buf);
        self.render_data_area(data_area, buf, state);

        if let Some(err) = state.get_error() {
            self.render_error_popup(area, buf, err);
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
