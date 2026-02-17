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
        let mut state_client = State::init();

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
        } else if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Up {
            state_client.update_list_item_index(KeyCode::Up).unwrap();
        } else if key_event.kind == KeyEventKind::Press && key_event.code == KeyCode::Down {
            state_client.update_list_item_index(KeyCode::Down).unwrap();
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
        let mut state = ListState::default();
        state.select(Some(index)); // select first item

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

#[derive(Default)]
struct State {
    exit: bool,
    list_item_index: usize,
}

pub struct StateClient {
    state_update_tx: Sender<StateActions>,
}

enum StateActions {
    UpdateExit(bool),
    ReadExit(SyncSender<bool>),
    UpdateListItemIndex(i8),
    ReadListItemIndex(SyncSender<usize>),
}

impl State {
    fn init() -> StateClient {
        let mut state = State {
            exit: false,
            list_item_index: 0,
        };
        let (state_update_tx, state_update_rx) = mpsc::channel::<StateActions>();

        thread::spawn(move || {
            loop {
                if let Err(err) = Self::handle_state_updates(&state_update_rx, &mut state) {
                    println!("Err: {:?}", err);
                }
            }
        });

        StateClient::new(state_update_tx)
    }

    fn handle_state_updates(
        state_update_rx: &Receiver<StateActions>,
        state: &mut State,
    ) -> Result<(), String> {
        while let Ok(action) = state_update_rx.recv() {
            match action {
                StateActions::UpdateExit(v) => state.exit = v,
                StateActions::ReadExit(sender) => {
                    if let Err(err) = sender.send(state.exit) {
                        return Err(err.to_string());
                    }
                }
                StateActions::UpdateListItemIndex(update) => {
                    let len = 3;
                    let list_item_index = ((state.list_item_index as isize + update as isize)
                        .rem_euclid(len as isize))
                        as usize;
                    state.list_item_index = list_item_index;
                }
                StateActions::ReadListItemIndex(sender) => {
                    if let Err(err) = sender.send(state.list_item_index) {
                        return Err(err.to_string());
                    }
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

    fn update_list_item_index(&self, code: KeyCode) -> Result<(), String> {
        let update = match code {
            KeyCode::Up => -1,
            KeyCode::Down => 1,
            _ => return Err(String::from("Foo")),
        };
        self.state_update_tx
            .send(StateActions::UpdateListItemIndex(update))
            .unwrap();

        Ok(())
    }

    fn read_exit(&self) -> bool {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx
            .send(StateActions::ReadExit(sender))
            .unwrap();
        let exit = receiver.recv().unwrap();
        exit
    }

    fn read_list_item_index(&self) -> usize {
        let (sender, receiver) = sync_channel(1);
        self.state_update_tx
            .send(StateActions::ReadListItemIndex(sender))
            .unwrap();
        let index = receiver.recv().unwrap();
        index
    }
}
