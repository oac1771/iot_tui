use crate::{
    pages::home::{View, ViewState, state::State},
    utils::spinner::Spinner,
};
use iot_sdk::{CharPropFlags, Uuid};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};

pub enum HomeWidget<'a> {
    PopUpError(PopUpErrorWidget<'a>),
    Display(DisplayWidget<'a>),
}

impl<'a> Widget for HomeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            HomeWidget::PopUpError(pop_up_error_widget) => pop_up_error_widget.render(area, buf),
            HomeWidget::Display(display_widget) => display_widget.render(area, buf),
        }
    }
}

pub struct DisplayWidget<'a> {
    state: &'a State,
    view: &'a View,
}

impl<'a> DisplayWidget<'a> {
    pub fn new(state: &'a State, view: &'a View) -> Self {
        Self { state, view }
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
            .unwrap_or(&vec![])
            .iter()
            .map(|c| ListItem::new(Line::from(format!("UUID: {}", c.uuid)).centered()))
            .collect::<Vec<ListItem>>();

        let index = self.state.get_characteristic_index();
        let mut characteristic_list_state = ListState::default();
        characteristic_list_state.select(Some(index));

        let mut list = List::new(characteristics_entry).block(block.clone());

        if let View::Characteristic(ViewState::Idle) = self.view {
            list = list.highlight_style(Style::new().bold().magenta());
        };

        StatefulWidget::render(list, area, buf, &mut characteristic_list_state);
    }

    fn render_peripheral_scan_spinner(
        &self,
        area: Rect,
        buf: &mut Buffer,
        spinner: &Spinner,
        block: &Block,
        scanning_message: &str,
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
            Line::raw(scanning_message),
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
        scanning_message: &str,
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
            Line::raw(scanning_message),
            Line::from(spinner.frame()).bold(),
        ])
        .alignment(Alignment::Center);

        paragraph.render(center_area, buf);
    }

    fn render_characteristic_response(
        &self,
        area: Rect,
        buf: &mut Buffer,
        block: &Block,
        response: &[u8],
        characteristic_id: Uuid,
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

        let response = String::from_utf8(response.to_owned()).unwrap_or(String::from("0.0"));

        let paragraph = Paragraph::new(response).alignment(Alignment::Center);

        paragraph.render(center_area, buf);
    }

    fn render_mid_area(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(40), Constraint::Percentage(60)]);
        let [left_area, right_area] = layout.areas(area);

        let peripheral_block = Block::bordered()
            .border_set(border::DOUBLE)
            .title_top(Line::from(" Peripherals ").centered());

        let characteristic_block = Block::bordered()
            .border_set(border::DOUBLE)
            .title_top(Line::from(" Characteristics ").centered());

        if let View::Peripheral(ViewState::Scanning((spinner, scanning_message))) = self.view {
            self.render_peripheral_scan_spinner(
                left_area,
                buf,
                spinner,
                &peripheral_block,
                scanning_message,
            )
        } else if !self.state.get_local_names().is_empty() {
            self.render_peripheral_names(left_area, buf, &peripheral_block)
        }

        if let View::Characteristic(ViewState::Scanning((spinner, scanning_message))) = self.view {
            self.render_characteristic_scan_spinner(
                right_area,
                buf,
                spinner,
                &characteristic_block,
                scanning_message,
            )
        } else if let View::Characteristic(ViewState::Payload(characteristic_id)) = self.view {
            if let Some(response) = self.state.get_characteristic_response(characteristic_id) {
                self.render_characteristic_response(
                    right_area,
                    buf,
                    &characteristic_block,
                    response,
                    *characteristic_id,
                );
            }
        } else if self.state.get_characteristics().is_some() {
            self.render_characteristics(right_area, buf, &characteristic_block)
        }

        peripheral_block.render(left_area, buf);
        characteristic_block.render(right_area, buf)
    }

    fn render_lower_area(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .flex(Flex::Center)
            .constraints(vec![
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ]);

        let [_, global_cmd_area, _] = layout.areas(area);

        let mut cmds =
            vec![Line::from(vec![" Quit: ".into(), " Ctrl + c ".blue().bold()]).centered()];

        if self.state.get_characteristics().is_some() {
            cmds.push(
                Line::from(vec![
                    " Navigate: ".into(),
                    " <Up/Down/Right/Left> ".blue().bold(),
                ])
                .centered(),
            );
        } else {
            cmds.push(
                Line::from(vec![" Navigate: ".into(), " <Up/Down> ".blue().bold()]).centered(),
            );
        }

        let global_cmds = Text::from(cmds);

        Paragraph::new(global_cmds).render(global_cmd_area, buf);
    }

    fn render_top_area(&self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .flex(Flex::Center)
            .constraints(vec![Constraint::Percentage(33), Constraint::Percentage(66)]);

        let [view_command_area, _] = layout.areas(area);

        let mut cmds = Vec::new();

        if let View::Peripheral(ViewState::Idle) = self.view {
            cmds.push(Line::from(vec![
                " Peripheral Scan: ".into(),
                " <s> ".blue().bold(),
            ]));

            if !self.state.get_local_names().is_empty() {
                cmds.push(
                    Line::from(vec![" Characteristic Scan: ".into(), " <c> ".blue().bold()])
                        .centered(),
                );
            }
        } else if let View::Characteristic(ViewState::Idle) = self.view
            && let Some(characteristic) = self.state.get_indexed_characteristic()
            && characteristic.properties.contains(CharPropFlags::READ)
        {
            cmds.push(Line::from(vec![" Read: ".into(), " <r> ".blue().bold()]).centered());
        } else if let View::Characteristic(ViewState::Payload(characteristic_id)) = self.view
            && self
                .state
                .get_characteristic_response(characteristic_id)
                .is_some()
        {
            cmds.push(
                Line::from(vec![
                    " Go back to Characteristic View: ".into(),
                    " <Esc> ".blue().bold(),
                ])
                .centered(),
            );
        }

        let view_specific_cmds = Text::from(cmds);

        Paragraph::new(view_specific_cmds.centered())
            .centered()
            .render(view_command_area, buf);
    }
}

impl<'a> Widget for DisplayWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let outline_block = Block::bordered()
            .border_set(border::DOUBLE)
            .border_style(Style::new().yellow())
            .title_top(
                Line::from("  IOT v0.0.0  ")
                    .style(Style::new().yellow())
                    .centered(),
            );

        let outline_block_inner_area = outline_block.inner(area);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ]);
        let [top_area, mid_area, lower_area] = layout.areas(outline_block_inner_area);

        self.render_lower_area(lower_area, buf);
        self.render_mid_area(mid_area, buf);
        self.render_top_area(top_area, buf);
        outline_block.render(area, buf);
    }
}

pub struct PopUpErrorWidget<'a> {
    error: &'a str,
}

impl<'a> PopUpErrorWidget<'a> {
    pub fn new(error: &'a str) -> Self {
        Self { error }
    }
}

impl<'a> Widget for PopUpErrorWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let instructions = Line::from(vec![" Exit ".into(), "<Esc> ".blue().bold()]);

        let block = Block::bordered()
            .title(Line::from("  Error!  ").bold().centered())
            .title_bottom(instructions.centered());

        let popup_area = popup_area(area, 80, 80);

        Clear::render(Clear, popup_area, buf);
        Paragraph::new(Line::raw(self.error))
            .block(block.clone())
            .red()
            .centered()
            .wrap(Wrap { trim: true })
            .render(popup_area, buf);

        block.render(popup_area, buf);
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
