use std::{net::SocketAddr, sync::Arc};

use bytes::Bytes;
use crossterm::ExecutableCommand;
use ratatui::buffer::Cell;
use ratatui::widgets::*;
use ratatui::{buffer::Buffer, widgets::Paragraph};
use tokio::sync::{mpsc, Mutex};

use crate::shared::{PeerData, UserInput};
use crate::{Result, Rx, Shared, TelnetTerminal};

pub struct Peer {
    pub rx: Rx,
    terminal: TelnetTerminal,
    last_buffer: Buffer,
}

impl Peer {
    pub async fn new(
        state: Arc<Mutex<Shared>>,
        addr: SocketAddr,
        terminal: TelnetTerminal,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        state.lock().await.add_peer(addr, tx);

        let size = terminal.size().unwrap();

        Peer {
            rx,
            terminal,
            last_buffer: Buffer::empty(size),
        }
    }

    pub async fn handle_input(&mut self, input: Bytes) -> Option<UserInput> {
        if &input[..] == b"\x1b" {
            return Some(UserInput::Quit);
        }

        match &input[..] {
            b"w" => Some(UserInput::MoveUp),
            b"a" => Some(UserInput::MoveLeft),
            b"d" => Some(UserInput::MoveRight),
            b"s" => Some(UserInput::MoveDown),
            _ => None,
        }
    }

    pub fn render(&mut self, state: &PeerData) -> Vec<u8> {
        let next_frame = self
            .terminal
            .draw(|f| {
                let main_paragraph =
                    Paragraph::new("Welcome to RMUD!").block(Block::new().borders(Borders::all()));
                f.render_widget(main_paragraph, f.size());
            })
            .unwrap();

        let changes = cells_to_bytes(&self.last_buffer.diff(&next_frame.buffer)).unwrap();

        self.last_buffer = next_frame.buffer.clone();

        changes
    }
}

fn cells_to_bytes(changes: &[(u16, u16, &Cell)]) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    for change in changes {
        buf.execute(crossterm::cursor::MoveTo(change.0, change.1))?
            .execute(crossterm::style::Print(change.2.symbol()))?;
    }

    Ok(buf)
}
