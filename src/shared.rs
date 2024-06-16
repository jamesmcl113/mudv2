use std::{collections::HashMap, net::SocketAddr};

use crate::{Result, RoomEvent, Tx};

pub enum UserInput {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Quit,
}

pub struct Shared {
    peers: HashMap<SocketAddr, PeerData>,
}

impl Shared {
    pub fn new() -> Self {
        Shared {
            peers: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, socket_addr: SocketAddr, tx: Tx) {
        self.peers.insert(
            socket_addr,
            PeerData {
                tx,
                state: PeerState::Playing,
                position: (0, 0),
            },
        );
    }

    pub fn move_peer(&mut self, socket_addr: &SocketAddr, input: UserInput) -> Result<()> {
        let peer = self.get_peer_data_mut(socket_addr);

        let (old_x, old_y) = peer.position;
        let new_pos = match input {
            UserInput::MoveUp => {
                if peer.position.1 == 0 {
                    (old_x, 0)
                } else {
                    (old_x, old_y - 1)
                }
            }
            UserInput::MoveDown => (old_x, old_y + 1),
            UserInput::MoveLeft => {
                if peer.position.0 == 0 {
                    (0, old_y)
                } else {
                    (old_x - 1, old_y)
                }
            }
            UserInput::MoveRight => (old_x + 1, old_y),
            _ => peer.position,
        };

        peer.position = new_pos;

        self.broadcast(RoomEvent::PeerMoved(
            self.peers.iter().map(|peer| peer.1.position).collect(),
        ));

        Ok(())
    }

    fn broadcast(&self, ev: RoomEvent) {
        for (_, data) in &self.peers {
            let _ = data.tx.send(ev.clone());
        }
    }

    pub fn get_peer_data_mut(&mut self, socket_addr: &SocketAddr) -> &mut PeerData {
        self.peers.get_mut(socket_addr).unwrap()
    }

    pub fn get_peer_data(&self, socket_addr: &SocketAddr) -> Option<&PeerData> {
        self.peers.get(&socket_addr)
    }

    pub fn remove_peer(&mut self, socket_addr: SocketAddr) {
        self.peers.remove(&socket_addr).unwrap();
    }
}

pub enum PeerState {
    Login,
    Playing,
}

pub struct PeerData {
    tx: Tx,
    state: PeerState,
    position: (usize, usize),
}
