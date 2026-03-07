tutorials: https://ratatui.rs/tutorials/

todo:
refactor to not have two panels, just show peripherals on scan
- on enter scan and show characteristics


        let info_block = Block::bordered()
            .title(Line::from("  ~~~~~~~~~~~~~  ").bold().centered())
            .border_set(border::DOUBLE);
        info_block.clone().render(area, buf);

        let inner = info_block.inner(area);

        if let Some(characteristic_spinner) = &self.characteristic_scan_spinner {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Length(3),
                    Constraint::Percentage(50),
                ])
                .split(inner);

            let center_area = layout[1];

            let line = if let Some(state) = &self.characteristic_scan_state {
                state
            } else {
                &String::from("Loading...")
            };

            let paragraph = Paragraph::new(vec![
                Line::raw(line),
                Line::from(characteristic_spinner.frame()).bold(),
            ])
            .alignment(Alignment::Center);

            paragraph.render(center_area, buf);
        } else {
            let characteristics = self.state.get_characteristics();
            let index = self.state.get_index();

            if !characteristics.is_empty() {
                let characteristics_entry = &self.state.get_characteristics()[index].to_string();
                Paragraph::new(Line::raw(characteristics_entry))
                    .block(info_block)
                    .centered()
                    .wrap(Wrap { trim: true })
                    .render(area, buf);
            }
        }