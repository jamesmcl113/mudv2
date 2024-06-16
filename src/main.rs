mod backend;
mod canvas;
mod peer;
mod shared;

use std::{collections::HashMap, error::Error, net::SocketAddr, sync::Arc};

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use ratatui::layout::Rect;
use ratatui::Terminal;
use shared::{PeerData, Shared};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_util::codec::{BytesCodec, Framed};

use crate::backend::TelnetBackend;
use crate::canvas::Canvas;
use crate::peer::Peer;
use crate::shared::UserInput;

type TelnetTerminal = Terminal<TelnetBackend>;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    let state = Arc::new(Mutex::new(Shared::new()));

    loop {
        let (stream, addr) = listener.accept().await?;
        let state = state.clone();

        tokio::spawn(async move {
            if let Err(e) = process(state, stream, addr).await {
                eprintln!("error while processing requests for {addr:?}, err = {e:?}");
            }
        });
    }
}

async fn process(state: Arc<Mutex<Shared>>, stream: TcpStream, addr: SocketAddr) -> Result<()> {
    let mut stream = Framed::new(stream, BytesCodec::new());

    // set no echo, character mode
    stream
        .send(Bytes::from_static(&[
            255, 253, 34, 255, 250, 34, 1, 0, 255, 240, 255, 251, 1,
        ]))
        .await?;

    // send NAWS
    stream.send(Bytes::from_static(&[255, 253, 31])).await?;

    let (width, height) = match stream.next().await {
        Some(Ok(bytes)) => get_telnet_size(bytes.as_ref())?,
        _ => {
            return Err("Failed to get options from client.".into());
        }
    };

    println!("Got terminal dimensions: w = {}, h = {}", width, height);

    let telnet_backend = TelnetBackend::new(Rect {
        x: 0,
        y: 0,
        width,
        height,
    });
    let terminal = Terminal::new(telnet_backend)?;

    // move these to some `ui` module
    let clear_bytes = canvas::clear_screen()?;
    stream.send(Bytes::from(clear_bytes)).await?;

    let mut peer = Peer::new(state.clone(), addr, terminal).await;
    let mut canvas = Canvas::new(width as usize, height as usize);

    loop {
        tokio::select! {
            Some(event) = peer.rx.recv() => {
                let render_bytes = handle_event(event, &mut canvas);
                stream.send(Bytes::from(render_bytes)).await.unwrap();
            }
            res = stream.next() => match res {
                Some(Ok(msg)) => {
                    if let Some(event) = peer.handle_input(msg.into()).await {
                        if matches!(event, UserInput::Quit) {
                            break;
                        } else {
                            state.lock().await.move_peer(&addr, event).unwrap();
                        }
                    }

                },
                Some(Err(e)) => {}
                None => break,
            }
        }
    }

    {
        let mut shared = state.lock().await;
        shared.remove_peer(addr);
    }

    let restore_bytes = canvas::restore_screen()?;
    stream.send(Bytes::from(restore_bytes)).await?;

    Ok(())
}

fn handle_event(ev: RoomEvent, canvas: &mut Canvas) -> Vec<u8> {
    let mut buffer = Vec::new();
    match ev {
        RoomEvent::PeerMoved(peer_positions) => canvas
            .redraw(&mut buffer, |ctx| {
                ctx.clear();
                for location in &peer_positions {
                    ctx.set_char('@', None, location.0, location.1)?;
                }

                Ok(())
            })
            .unwrap(),
    }

    buffer
}

fn get_telnet_size(bytes: &[u8]) -> Result<(u16, u16)> {
    let len = bytes.len();

    // get the naws negotiation
    // this assumes that the NAWS negotiation always comes last...is this correct?
    let naws = &bytes[len - 9..];

    let width = (naws[3] as u16) << 8 | naws[4] as u16;
    let height = (naws[5] as u16) << 8 | naws[6] as u16;

    Ok((width, height))
}

#[derive(Clone)]
pub enum RoomEvent {
    PeerMoved(Vec<(usize, usize)>),
}

pub type Rx = tokio::sync::mpsc::UnboundedReceiver<RoomEvent>;
pub type Tx = tokio::sync::mpsc::UnboundedSender<RoomEvent>;
