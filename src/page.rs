use std::{
    io,
    sync::mpsc::{self, Receiver, Sender, SyncSender, sync_channel},
    thread,
};

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
        let state_client = State::init();

        let (events_tx, events_rx) = mpsc::channel::<Event>();
        thread::spawn(move || {
            handle_input_events(events_tx).unwrap();
        });

        while !state_client.read_exit() {
            terminal.draw(|frame| self.draw(frame))?;
            match events_rx.recv().unwrap() {
                Event::UserInput(key_event) => self.handle_key_event(key_event, &state_client)?,
            }
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        state_client: &StateClient,
    ) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Char('q') {
            state_client.update_exit(true);
        }
        // } else if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Up {

        // } else if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Down {

        // }

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
    }
}

fn handle_input_events(events_tx: Sender<Event>) -> io::Result<()> {
    loop {
        match event::read().unwrap() {
            CrosstermEvent::Key(key_event) => events_tx.send(Event::UserInput(key_event)).unwrap(),
            _ => {}
        }
    }
}

#[derive(Default)]
struct State {
    exit: bool,
}

struct StateClient {
    state_update_tx: Sender<StateActions>,
}

enum StateActions {
    UpdateExit(bool),
    ReadExit(SyncSender<bool>),
}

impl State {
    fn init() -> StateClient {
        let state = State { exit: false };
        let (state_update_tx, state_update_rx) = mpsc::channel::<StateActions>();

        thread::spawn(move || {
            Self::handle_state_updates(state_update_rx, state).unwrap();
        });

        StateClient::new(state_update_tx)
    }

    fn handle_state_updates(
        state_update_rx: Receiver<StateActions>,
        mut state: State,
    ) -> io::Result<()> {
        while let Ok(action) = state_update_rx.recv() {
            match action {
                StateActions::UpdateExit(v) => state.exit = v,
                StateActions::ReadExit(sender) => {
                    let _ = sender.send(state.exit);
                }
            }
        }

        Ok(())
    }
}

impl StateClient {
    fn new(state_update_tx: Sender<StateActions>) -> Self {
        Self { state_update_tx }
    }

    fn update_exit(&self, exit: bool) {
        self.state_update_tx
            .send(StateActions::UpdateExit(exit))
            .unwrap();
    }

    fn read_exit(&self) -> bool {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx
            .send(StateActions::ReadExit(sender))
            .unwrap();
        let exit = receiver.recv().unwrap();
        exit
    }
}
