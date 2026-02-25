use crate::utils::spinner::Spinner;

use super::{scan::ScanCmd, state::State};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};
use tokio::sync::mpsc::Sender;

pub struct HomePage {
    state: State,
    error: Option<String>,
    scan_spinner: Option<Spinner>,
    home_page_event_tx: Sender<HomePageEvent>,
}

pub enum HomePageEvent {
    Pending,
    Complete(Vec<String>),
    Error(String),
}

impl HomePage {
    pub fn new(home_page_event_tx: Sender<HomePageEvent>) -> Self {
        Self {
            state: State::default(),
            error: None,
            scan_spinner: None,
            home_page_event_tx,
        }
    }

    pub async fn tick(&mut self) {
        if let Some(spinner) = &mut self.scan_spinner {
            spinner.tick();
        }
    }

    pub async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<(), String> {
        if key_event.kind == KeyEventKind::Press && self.error.is_none() {
            match key_event.code {
                KeyCode::Char('s') => self.scan().await?,
                KeyCode::Up => self.state.update_scan_items_index(-1),
                KeyCode::Down => self.state.update_scan_items_index(1),
                _ => {}
            }
        } else if key_event.kind == KeyEventKind::Press && self.error.is_some() {
            if let KeyCode::Char('q') = key_event.code {
                self.error = None
            }
        }

        Ok(())
    }

    pub async fn handle_home_page_event(
        &mut self,
        home_page_event: HomePageEvent,
    ) -> Result<(), String> {
        match home_page_event {
            HomePageEvent::Pending => self.scan_spinner = Some(Spinner::default()),
            HomePageEvent::Complete(scan_items) => {
                self.scan_spinner = None;
                self.state.update_scan_items(scan_items);
            }
            HomePageEvent::Error(err) => {
                self.error = Some(err);
            }
        }
        Ok(())
    }

    async fn scan(&self) -> Result<(), String> {
        ScanCmd::handle(&self.home_page_event_tx).await?;
        Ok(())
    }

    fn render_title_area(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(35), Constraint::Percentage(65)]);

        let [cmd_area, meta_data_area] = layout.areas(area);

        let instructions = Line::from(vec![
            " Quit ".into(),
            "<Ctrl + c> ".blue().bold(),
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

    fn render_data_area(&self, area: Rect, buf: &mut Buffer) {
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

        list_block.clone().render(list_area, buf);

        let inner = list_block.inner(list_area);

        if let Some(scan_spinner) = &self.scan_spinner {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Length(3), // height of spinner section
                    Constraint::Percentage(50),
                ])
                .split(inner);

            let center_area = layout[1];

            let paragraph = Paragraph::new(vec![
                Line::raw("Scanning..."),
                Line::from(scan_spinner.frame()).bold(),
            ])
            .alignment(Alignment::Center);

            paragraph.render(center_area, buf);
        } else {
            let (scan_items_index, scan_items) = self.state.get_scan_items();

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
    }

    fn render_error_popup(&self, area: Rect, buf: &mut Buffer, err: &str) {
        let instructions = Line::from(vec![" Exit ".into(), "<q> ".blue().bold()]);

        let block = Block::bordered()
            .title(Line::from("  Error!  ").bold().centered())
            .title_bottom(instructions.centered());

        let popup_area = Self::popup_area(area, 80, 80);

        Clear::render(Clear, popup_area, buf);
        Paragraph::new(Line::raw(err))
            .block(block.clone())
            .red()
            .centered()
            .wrap(Wrap { trim: true })
            .render(popup_area, buf);

        block.render(popup_area, buf);
    }

    fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }
}

impl Widget for &HomePage {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)]);
        let [title_area, data_area] = layout.areas(area);

        self.render_title_area(title_area, buf);
        self.render_data_area(data_area, buf);

        if let Some(err) = &self.error {
            self.render_error_popup(area, buf, err);
        }
    }
}
