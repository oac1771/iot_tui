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
    error: Option<String>,
    view: View,
    home_page_event_tx: Sender<HomePageEvent>,
    peripheral_scan_spinner: Option<Spinner>,
    characteristic_scan_spinner: Option<Spinner>,
}

pub enum HomePageEvent {
    PeripheralScanStarted,
    PeripheralScanComplete(Vec<String>),
    PeripheralScanError(String),
    CharacteristicScanStarted,
    CharacteristicScanComplete(Vec<Characteristic>),
    CharacteristicScanError(String),
}

enum View {
    PeripheralList,
    CharacteristicList,
}

impl HomePage {
    pub fn new(home_page_event_tx: Sender<HomePageEvent>) -> Self {
        Self {
            state: State::default(),
            error: None,
            view: View::PeripheralList,
            home_page_event_tx,
            peripheral_scan_spinner: None,
            characteristic_scan_spinner: None,
        }
    }

    pub async fn tick(&mut self) {
        if let Some(spinner) = &mut self.peripheral_scan_spinner {
            spinner.tick()
        } else if let Some(spinner) = &mut self.characteristic_scan_spinner {
            spinner.tick()
        }
    }

    pub async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<(), String> {
        if key_event.kind == KeyEventKind::Press && self.error.is_none() {
            match key_event.code {
                KeyCode::Char('s') => self.get_peripherals().await?,
                KeyCode::Enter
                    if !self.state.get_local_names().is_empty()
                        && self.characteristic_scan_spinner.is_none() =>
                {
                    if let KeyCode::Enter = key_event.code {
                        let local_names = self.state.get_local_names();
                        let index = self.state.get_index();
                        let local_name = &local_names[index];
                        self.get_characteristics(local_name).await?
                    }
                }
                KeyCode::Up => self.state.update_index(-1),
                KeyCode::Down => self.state.update_index(1),
                _ => {}
            }
        } else if key_event.kind == KeyEventKind::Press && self.error.is_some() {
            if let KeyCode::Char('q') = key_event.code {
                self.error = None
            }
        }

        Ok(())
    }

    pub async fn handle_page_event(&mut self, event: HomePageEvent) -> Result<(), String> {
        match event {
            HomePageEvent::PeripheralScanStarted => {
                self.peripheral_scan_spinner = Some(Spinner::default());
            }
            HomePageEvent::PeripheralScanComplete(local_names) => {
                self.peripheral_scan_spinner = None;
                self.view = View::PeripheralList;
                self.state.clear_characteristics(local_names.len());
                self.state.update_local_names(local_names);
            }
            HomePageEvent::PeripheralScanError(err) => {
                self.peripheral_scan_spinner = None;
                self.error = Some(err);
            }
            HomePageEvent::CharacteristicScanStarted => {
                self.characteristic_scan_spinner = Some(Spinner::default())
            }
            HomePageEvent::CharacteristicScanComplete(characteristics) => {
                self.characteristic_scan_spinner = None;
                self.view = View::CharacteristicList;
                self.state.update_characteristics(characteristics);
            }
            HomePageEvent::CharacteristicScanError(err) => {
                self.characteristic_scan_spinner = None;
                self.error = Some(err);
            }
        }
        Ok(())
    }

    async fn get_peripherals(&self) -> Result<(), String> {
        Peripherals::get_peripherals(&self.home_page_event_tx).await?;
        Ok(())
    }

    async fn get_characteristics(&self, local_name: &str) -> Result<(), String> {
        Peripherals::get_characteristics(&self.home_page_event_tx, local_name).await?;
        Ok(())
    }

    fn render_title_area(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(35), Constraint::Percentage(65)]);

        let [cmd_area, meta_data_area] = layout.areas(area);

        let instructions = Line::from(vec![" Quit ".into(), "<Ctrl + c> ".blue().bold()]);

        let cmd_block = Block::bordered()
            .title(Line::from("  Commands  ").bold().centered())
            .border_set(border::DOUBLE)
            .title_bottom(instructions.centered());

        Block::bordered()
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

    fn render_peripheral_names(&self, area: Rect, buf: &mut Buffer, block: &Block) {
        let local_names = self.state.get_local_names();
        let index = self.state.get_index();

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
        let characteristics = self.state.get_characteristics();

        if !characteristics.is_empty() {
            let mut characteristic_list_state = ListState::default();
            characteristic_list_state.select(Some(0));

            let characteristics_entry = characteristics
                .iter()
                .map(|c| ListItem::new(Line::from(c.to_string()).alignment(Alignment::Center)))
                .collect::<Vec<ListItem>>();

            StatefulWidget::render(
                List::new(characteristics_entry)
                    .block(block.clone())
                    .highlight_style(Style::new().bold().green()),
                area,
                buf,
                &mut characteristic_list_state,
            );
        }
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

        data_block.clone().render(area, buf);

        if let Some(peripheral_scan_spinner) = &self.peripheral_scan_spinner {
            self.render_peripheral_scan_spinner(area, buf, peripheral_scan_spinner, &data_block)
        } else if let Some(characteristic_scan_spinner) = &self.characteristic_scan_spinner {
            self.render_characteristic_scan_spinner(
                area,
                buf,
                characteristic_scan_spinner,
                &data_block,
            )
        } else {
            match self.view {
                View::PeripheralList => self.render_peripheral_names(area, buf, &data_block),
                View::CharacteristicList => self.render_characteristics(area, buf, &data_block),
            }
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

        if let Some(err) = &self.error {
            self.render_error_popup(area, buf, err);
        } else {
            self.render_title_area(title_area, buf);
            self.render_data_area(data_area, buf);
        }
    }
}
