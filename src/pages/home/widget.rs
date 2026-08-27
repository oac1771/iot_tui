use crate::{
    pages::home::{View, ViewState, state::State},
    utils::{
        peripherals::{CharacteristicType, KnownCharacteristic},
        spinner::Spinner,
    },
};
use iot_sdk::CharPropFlags;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget, Wrap},
};

// use crossbeam::channel::TryRecvError;

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
    view: &'a mut View,
}

impl<'a> DisplayWidget<'a> {
    pub fn new(state: &'a State, view: &'a mut View) -> Self {
        Self { state, view }
    }

    fn render_peripheral_names(&mut self, area: Rect, buf: &mut Buffer, block: &Block) {
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

    fn render_characteristics(&mut self, area: Rect, buf: &mut Buffer, block: &Block) {
        let characteristic_entries = self
            .state
            .get_characteristics()
            .unwrap_or(&vec![])
            .iter()
            .map(|characteristic| {
                ListItem::new(
                    Line::from(characteristic.display_characteristic_properties()).centered(),
                )
            })
            .collect::<Vec<ListItem>>();

        let index = self.state.get_characteristic_index();
        let mut characteristic_list_state = ListState::default();
        characteristic_list_state.select(Some(index));

        let mut list = List::new(characteristic_entries).block(block.clone());

        if let View::Characteristic(ViewState::Idle) = self.view {
            list = list.highlight_style(Style::new().bold().magenta());
        };

        StatefulWidget::render(list, area, buf, &mut characteristic_list_state);
    }

    fn render_peripheral_scan_spinner(
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
        area: Rect,
        buf: &mut Buffer,
        block: &Block,
        response: &[u8],
        characteristic: &KnownCharacteristic,
    ) -> Result<(), String> {
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

        match characteristic.handle_response(response) {
            Ok(response) => {
                let text = Text::from(response);
                let paragraph = Paragraph::new(text).alignment(Alignment::Center);
                paragraph.render(center_area, buf);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn render_notification_response(
        area: Rect,
        buf: &mut Buffer,
        block: &Block,
        data: &[u8],
        _characteristic: &KnownCharacteristic,
    ) -> Result<(), String> {
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

        let lines = vec![Line::from(vec![format!("Data: {:?}", data).into()])];

        let text = Text::from(lines);

        let payload_block = Block::bordered()
            .border_set(border::DOUBLE)
            .border_style(Style::new().light_blue())
            .title_top(
                Line::from("  Notifications  ")
                    .style(Style::new().light_blue())
                    .centered(),
            );

        Paragraph::new(text)
            .block(payload_block)
            .render(center_area, buf);

        Ok(())
    }

    fn render_mid_area(&mut self, area: Rect, buf: &mut Buffer) {
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

        match self.view {
            View::Peripheral(view_state) => match view_state {
                ViewState::Scanning((spinner, scanning_message)) => {
                    Self::render_peripheral_scan_spinner(
                        left_area,
                        buf,
                        spinner,
                        &peripheral_block,
                        scanning_message,
                    )
                }
                ViewState::Idle => self.render_peripheral_names(left_area, buf, &peripheral_block),
                _ => {}
            },
            _ => self.render_peripheral_names(left_area, buf, &peripheral_block),
        }

        match self.view {
            View::Characteristic(view_state) => match view_state {
                ViewState::Scanning((spinner, scanning_message)) => {
                    Self::render_characteristic_scan_spinner(
                        right_area,
                        buf,
                        spinner,
                        &characteristic_block,
                        scanning_message,
                    )
                }
                ViewState::Payload(characteristic_id) => {
                    if let Some(characteristic) = self.state.get_characteristic(characteristic_id)
                        && let Some(response) =
                            self.state.get_characteristic_response(characteristic_id)
                        && let Err(err) = Self::render_characteristic_response(
                            right_area,
                            buf,
                            &characteristic_block,
                            response.data(),
                            characteristic,
                        )
                    {
                        let error = PopUpErrorWidget::new(&err);
                        error.render(right_area, buf);
                    }
                }
                ViewState::Notifying((_notification_rx, _notifications)) => {
                    if let Some(_characteristic) = self.state.get_indexed_characteristic() {
                        // notifications.push(String::from("notification..."));

                        // let result = match notification_rx.try_recv() {
                        //     Ok(value) => {
                        //         String::from("Value")
                        //     }
                        //     Err(TryRecvError::Empty) => {
                        //         String::from("Empty")
                        //     }
                        //     Err(TryRecvError::Disconnected) => {
                        //         String::from("Disconnected")
                        //     }
                        // };

                        // println!("{:?}", notifications);

                        // if let Err(err) = Self::render_notification_response(
                        //     right_area,
                        //     buf,
                        //     &characteristic_block,
                        //     &value.value,
                        //     characteristic,
                        // ) {
                        //     let error = PopUpErrorWidget::new(&err);
                        //     error.render(right_area, buf);
                        // }
                    }
                }
                ViewState::Idle => {
                    self.render_characteristics(right_area, buf, &characteristic_block)
                }
                _ => {}
            },
            _ => self.render_characteristics(right_area, buf, &characteristic_block),
        }

        peripheral_block.render(left_area, buf);
        characteristic_block.render(right_area, buf)
    }

    fn render_lower_area(&mut self, area: Rect, buf: &mut Buffer) {
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

    fn render_top_area(&mut self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .flex(Flex::Center)
            .constraints(vec![Constraint::Percentage(33), Constraint::Percentage(66)]);

        let [view_command_area, descriptor_area] = layout.areas(area);

        match (&mut self.view, self.state.get_indexed_characteristic()) {
            (View::Peripheral(ViewState::Idle), None) => {
                let mut cmds = Vec::new();

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

                let view_specific_cmds = Text::from(cmds);

                Paragraph::new(view_specific_cmds.centered())
                    .centered()
                    .render(view_command_area, buf);
            }
            (View::Characteristic(ViewState::Idle), Some(characteristic)) => {
                let mut cmds = Vec::new();

                let descriptors = Line::from(
                    characteristic
                        .descriptors()
                        .filter_map(|d| {
                            if !d.metadata().is_empty() {
                                Some(Span::raw(format!("{:?}", d.metadata())))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Span>>(),
                )
                .centered();
                let view_descriptors = Text::from(descriptors);

                Paragraph::new(view_descriptors.centered())
                    .centered()
                    .render(descriptor_area, buf);

                characteristic.properties().iter().for_each(|p| {
                    if p.contains(CharPropFlags::READ) {
                        cmds.push(
                            Line::from(vec![" Read: ".into(), " <r> ".blue().bold()]).centered(),
                        );
                    } else if p.contains(CharPropFlags::NOTIFY) {
                        cmds.push(
                            Line::from(vec![" Notify: ".into(), " <n> ".blue().bold()]).centered(),
                        );
                    } else if p.contains(CharPropFlags::WRITE) {
                        cmds.push(
                            Line::from(vec![" Write: ".into(), " <w> ".blue().bold()]).centered(),
                        );
                    }
                });

                let view_specific_cmds = Text::from(cmds);

                Paragraph::new(view_specific_cmds.centered())
                    .centered()
                    .render(view_command_area, buf);
            }
            (View::Characteristic(ViewState::Payload(characteristic_id)), _) => {
                let mut cmds = Vec::new();

                if self
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
            (View::Characteristic(ViewState::Editing), Some(characteristic)) => {
                let mut lines = vec![
                    Line::from(vec![
                        "Press ".into(),
                        "Esc".bold(),
                        " to exit editing mode".into(),
                    ]),
                    Line::from(vec![
                        "Press ".into(),
                        "Enter".bold(),
                        " to submit write request".into(),
                    ]),
                ];

                if let CharacteristicType::Unknown = characteristic.characteristic_type() {
                    lines.push(Line::from(vec![
                        "Unknown Characteristic".red().bold(),
                        "cannot validate write data".red().bold(),
                    ]));
                }

                let text = Text::from(lines);
                Paragraph::new(text)
                    .centered()
                    .render(view_command_area, buf);

                let payload_block = Block::bordered()
                    .border_set(border::DOUBLE)
                    .border_style(Style::new().light_green())
                    .title_top(
                        Line::from("  Payload  ")
                            .style(Style::new().light_green())
                            .centered(),
                    );

                Paragraph::new(self.state.input.value.as_str())
                    .block(payload_block)
                    .render(descriptor_area, buf);
            }
            _ => {}
        }
    }
}

impl<'a> Widget for DisplayWidget<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
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

        // let state = self.state;
        // let mut view = self.view;

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
        let lines: Vec<Line> = self.error.split('\n').map(Line::from).collect();

        Clear::render(Clear, popup_area, buf);
        Paragraph::new(lines)
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
