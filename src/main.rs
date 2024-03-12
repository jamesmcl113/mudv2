use std::{collections::HashMap, error::Error, net::SocketAddr, sync::Arc};

use bytes::{Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio_util::codec::{BytesCodec, Framed, LengthDelimitedCodec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    let state = Arc::new(Mutex::new(Shared {
        peers: HashMap::new(),
    }));

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

fn get_ansi_bytes(command: impl crossterm::Command) -> BytesMut {
    let mut buf = BytesMut::new();
    command.write_ansi(&mut buf).unwrap();

    buf
}

async fn process(
    state: Arc<Mutex<Shared>>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let mut stream = Framed::new(stream, BytesCodec::new());
    let mut peer = Peer::new(state, addr).await;

    // set no echo, character mode
    stream
        .send(Bytes::from_static(&[
            255, 253, 34, 255, 250, 34, 1, 0, 255, 240, 255, 251, 1,
        ]))
        .await?;

    // send NAWS
    stream.send(Bytes::from_static(&[255, 253, 31])).await?;

    // this assumes that the NAWS negotiation always comes last...is this correct?
    let (width, height) = match stream.next().await {
        Some(Ok(bytes)) => get_telnet_size(bytes.as_ref())?,
        _ => {
            return Err("Failed to get options from client.".into());
        }
    };

    println!("Got terminal dimensions: w = {}, h = {}", width, height);

    loop {
        tokio::select! {
            Some(msg) = peer.rx.recv() => {
                stream.send(Bytes::from(msg)).await?;
            }
            res = stream.next() => match res {
                Some(Ok(msg)) => {
                    println!("{:?}", msg);
                    if &msg[..] == b"\x1b" {
                        break;
                    }
                },
                Some(Err(e)) => {}
                None => break,
            }
        }
    }

    Ok(())
}

fn get_telnet_size(bytes: &[u8]) -> Result<(u16, u16), Box<dyn Error>> {
    let len = bytes.len();

    // get the naws negotiation
    let naws = &bytes[len - 9..];

    let width = (naws[3] as u16) << 8 | naws[4] as u16;
    let height = (naws[5] as u16) << 8 | naws[6] as u16;

    Ok((width, height))
}

type Rx = tokio::sync::mpsc::UnboundedReceiver<String>;
type Tx = tokio::sync::mpsc::UnboundedSender<String>;

struct Peer {
    rx: Rx,
}

impl Peer {
    async fn new(state: Arc<Mutex<Shared>>, addr: SocketAddr) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        state.lock().await.peers.insert(addr, tx);

        Peer { rx }
    }
}

struct Shared {
    peers: HashMap<SocketAddr, Tx>,
}
