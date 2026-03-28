use crate::utils::spinner::Spinner;

use super::{State, peripherals::Peripherals};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use iot_sdk::Characteristic;
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
    view: View,
    error: Option<String>,
    home_page_event_tx: Sender<HomePageEvent>,
    peripherals: Peripherals,
}

pub enum HomePageEvent {
    PeripheralScanStarted,
    PeripheralScanComplete(Vec<String>),
    PeripheralScanError(String),
    CharacteristicScanStarted,
    CharacteristicScanComplete(Vec<Characteristic>),
    CharacteristicScanError(String),
    CharacteristicCallStarted,
}

enum View {
    Peripheral(ViewState),
    Characteristic(ViewState),
}

enum ViewState {
    Idle,
    Scanning(Spinner),
}

impl HomePage {
    pub async fn new(home_page_event_tx: Sender<HomePageEvent>) -> Result<Self, String> {
        let peripherals = Peripherals::new().await?;
        let home_page = Self {
            state: State::default(),
            error: None,
            view: View::Peripheral(ViewState::Idle),
            home_page_event_tx,
            peripherals,
        };

        Ok(home_page)
    }

    pub async fn tick(&mut self) {
        match &mut self.view {
            View::Peripheral(ViewState::Scanning(spinner)) => spinner.tick(),
            View::Characteristic(ViewState::Scanning(spinner)) => spinner.tick(),
            _ => {}
        }
    }

    pub async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<(), String> {
        if key_event.kind == KeyEventKind::Press && self.error.is_none() {
            match self.view {
                View::Peripheral(ViewState::Idle) => match key_event.code {
                    KeyCode::Char('s') => {
                        self.peripherals
                            .get_peripherals(&self.home_page_event_tx)
                            .await?
                    }
                    KeyCode::Enter if !self.state.get_local_names().is_empty() => {
                        let local_names = self.state.get_local_names();
                        let index = self.state.get_peripheral_index();
                        let local_name = &local_names[index];
                        self.get_characteristics(local_name).await?
                    }
                    KeyCode::Up => self.state.update_peripheral_index(-1),
                    KeyCode::Down => self.state.update_peripheral_index(1),
                    _ => {}
                },
                View::Characteristic(ViewState::Idle) => match key_event.code {
                    KeyCode::Esc => self.view = View::Peripheral(ViewState::Idle),
                    KeyCode::Up => self.state.update_characteristic_index(-1),
                    KeyCode::Down => self.state.update_characteristic_index(1),
                    // KeyCode::Enter if !self.state.get_characteristics().is_empty() => {
                    //     let characteristics = self.state.get_characteristics();
                    //     let index = self.state.get_characteristic_index();
                    //     let characteristic = &characteristics[index];
                    //     self.call_characteristic(characteristic).await?
                    // }
                    _ => {}
                },
                _ => {}
            }
        } else if key_event.kind == KeyEventKind::Press && self.error.is_some() {
            if let KeyCode::Esc = key_event.code {
                self.error = None
            }
        }

        Ok(())
    }

    pub async fn handle_page_event(&mut self, event: HomePageEvent) -> Result<(), String> {
        match event {
            HomePageEvent::PeripheralScanStarted => {
                self.view = View::Peripheral(ViewState::Scanning(Spinner::default()))
            }
            HomePageEvent::PeripheralScanComplete(local_names) => {
                self.view = View::Peripheral(ViewState::Idle);
                self.state.clear_characteristics(local_names.len());
                self.state.update_local_names(local_names);
            }
            HomePageEvent::PeripheralScanError(err) => {
                self.view = View::Peripheral(ViewState::Idle);
                self.error = Some(err);
            }
            HomePageEvent::CharacteristicScanStarted => {
                self.view = View::Characteristic(ViewState::Scanning(Spinner::default()))
            }
            HomePageEvent::CharacteristicScanComplete(characteristics) => {
                self.view = View::Characteristic(ViewState::Idle);
                self.state.update_characteristics(characteristics);
            }
            HomePageEvent::CharacteristicScanError(err) => {
                self.view = View::Peripheral(ViewState::Idle);
                self.error = Some(err);
            }
            HomePageEvent::CharacteristicCallStarted => {
                self.view = View::Characteristic(ViewState::Scanning(Spinner::default()))
            }
        }
        Ok(())
    }

    async fn get_characteristics(&self, local_name: &str) -> Result<(), String> {
        let characteristics = self.state.get_characteristics();
        if characteristics.is_empty() {
            self.peripherals
                .get_characteristics(&self.home_page_event_tx, local_name)
                .await?;
        } else {
            let _ = self
                .home_page_event_tx
                .send(HomePageEvent::CharacteristicScanComplete(
                    characteristics.clone(),
                ))
                .await;
        }

        Ok(())
    }

    // async fn call_characteristic(&self, characteristic: &Characteristic) -> Result<(), String> {
    //     Peripherals::call_characteristic(&self.home_page_event_tx, characteristic).await?;
    //     Ok(())
    // }

    fn render_title_area(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(35), Constraint::Percentage(65)]);

        let [cmd_area, meta_data_area] = layout.areas(area);

        let instructions = Line::from(vec![" Quit ".into(), "<Ctrl + c> ".blue().bold()]);

        let cmd_block = Block::bordered()
            .title(Line::from("  Commands  ").bold().centered())
            .title_bottom(instructions.centered())
            .border_set(border::DOUBLE);

        let cmds = Line::from(vec![" Scan ".into(), "<s>".blue().bold()]);

        Paragraph::new(cmds)
            .block(cmd_block)
            .white()
            .left_aligned()
            .wrap(Wrap { trim: true })
            .render(cmd_area, buf);

        let meta_data_block = Block::bordered().border_set(border::DOUBLE);

        let view_specific_cmds = match self.view {
            View::Peripheral(_) => Line::from(vec![
                " View Characteristics ".into(),
                " <Enter> ".blue().bold(),
                " Up ".into(),
                " <Up> ".blue().bold(),
                " Down ".into(),
                " <Down> ".blue().bold(),
            ]),
            View::Characteristic(_) => Line::from(vec![
                " Go pack to peripheral view ".into(),
                "<Esc>".blue().bold(),
            ]),
        };

        Paragraph::new(view_specific_cmds)
            .block(meta_data_block)
            .white()
            .centered()
            .wrap(Wrap { trim: true })
            .render(meta_data_area, buf);
    }

    fn render_peripheral_names(&self, area: Rect, buf: &mut Buffer, block: &Block) {
        let local_names = self.state.get_local_names();
        let index = self.state.get_peripheral_index();

        let scan_list: Vec<ListItem> = local_names
            .iter()
            .map(|p| ListItem::new(Line::from(p.as_str()).alignment(Alignment::Center)))
            .collect();

        let mut scan_list_state = ListState::default();
        scan_list_state.select(Some(index));

        StatefulWidget::render(
            List::new(scan_list)
                .block(block.clone())
                .highlight_style(Style::new().bold().green()),
            area,
            buf,
            &mut scan_list_state,
        );
    }

    fn render_characteristics(&self, area: Rect, buf: &mut Buffer, block: &Block) {
        let characteristics_entry = self
            .state
            .get_characteristics()
            .iter()
            .map(|c| ListItem::new(Line::from(c.to_string()).alignment(Alignment::Center)))
            .collect::<Vec<ListItem>>();

        let index = self.state.get_characteristic_index();
        let mut characteristic_list_state = ListState::default();
        characteristic_list_state.select(Some(index));

        StatefulWidget::render(
            List::new(characteristics_entry)
                .block(block.clone())
                .highlight_style(Style::new().bold().green()),
            area,
            buf,
            &mut characteristic_list_state,
        );
    }

    fn render_peripheral_scan_spinner(
        &self,
        area: Rect,
        buf: &mut Buffer,
        spinner: &Spinner,
        block: &Block,
    ) {
        let inner = block.inner(area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(3),
                Constraint::Percentage(50),
            ])
            .split(inner);

        let center_area = layout[1];

        let paragraph = Paragraph::new(vec![
            Line::raw("Scanning For Peripherals..."),
            Line::from(spinner.frame()).bold(),
        ])
        .alignment(Alignment::Center);

        paragraph.render(center_area, buf);
    }

    fn render_characteristic_scan_spinner(
        &self,
        area: Rect,
        buf: &mut Buffer,
        spinner: &Spinner,
        block: &Block,
    ) {
        let inner = block.inner(area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(3),
                Constraint::Percentage(50),
            ])
            .split(inner);

        let center_area = layout[1];

        let paragraph = Paragraph::new(vec![
            Line::raw("Scanning For Characteristics..."),
            Line::from(spinner.frame()).bold(),
        ])
        .alignment(Alignment::Center);

        paragraph.render(center_area, buf);
    }

    fn render_data_area(&self, area: Rect, buf: &mut Buffer) {
        let data_block = Block::bordered()
            .title(Line::from(" ***** ").bold().centered())
            .border_set(border::DOUBLE);

        match &self.view {
            View::Peripheral(ViewState::Idle) => {
                self.render_peripheral_names(area, buf, &data_block)
            }
            View::Peripheral(ViewState::Scanning(spinner)) => {
                self.render_peripheral_scan_spinner(area, buf, spinner, &data_block)
            }
            View::Characteristic(ViewState::Idle) => {
                self.render_characteristics(area, buf, &data_block)
            }
            View::Characteristic(ViewState::Scanning(spinner)) => {
                self.render_characteristic_scan_spinner(area, buf, spinner, &data_block)
            }
        }

        data_block.render(area, buf);
    }

    fn render_error_popup(&self, area: Rect, buf: &mut Buffer, err: &str) {
        let instructions = Line::from(vec![" Exit ".into(), "<Esc> ".blue().bold()]);

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

        if let Some(err) = &self.error {
            self.render_error_popup(area, buf, err);
        } else {
            self.render_title_area(title_area, buf);
            self.render_data_area(data_area, buf);
        }
    }
}
