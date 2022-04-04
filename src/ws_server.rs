#[allow(unused_imports)]
use crate::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{future, StreamExt, TryStreamExt};

#[derive(Debug)]
pub struct WSServer {
	addr: String,
	listener: Option<TcpListener>,
	open: bool
}

impl Default for WSServer {
	fn default() -> Self {
		WSServer::new(DEFAULT_WS_ADDR)
	}
}

impl WSServer {
	pub fn new(addr: &str) -> WSServer {
		WSServer{
			addr: addr.to_string(),
			listener: None,
			open: false
		}
	}

	pub async fn prepare(&mut self) -> DVResult<()> {
		let try_socket = TcpListener::bind(&self.addr).await;
		self.listener = match try_socket {
			Ok(v) => {
				info!("Bound on {}", self.addr);
				Some(v)
			},
			Err(err) => {
				error!("Failed to bind to {}: {}", self.addr, err);
				return Err(err)?
			}
		};
		self.open = true;
		Ok(())
	}

	pub async fn main_loop(&self) -> DVResult<()> {
		let listener = match &self.listener {
			Some(v) => v,
			None => return Err(DVError::NotReady("run WSServer.prepare() first".to_string()))?
		};
		info!("Listening on {}", self.addr);
		while self.open {
			match listener.accept().await {
				Ok((stream, socket_addr)) => {
					info!("stream = {:?}, socket_addr = {:?}", stream, socket_addr);
					tokio::spawn(accept_connection(stream));
				},
				Err(err) => {
					error!("Failed to accept incoming connection: {:?}", err);
				}
			};
	    }
	    info!("Stopped listening on {}", self.addr);
	    Ok(())
	}
}

async fn accept_connection(stream: TcpStream) {
	let addr = stream.peer_addr().expect("connected streams should have a peer address");
    info!("Peer address: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (write, read) = ws_stream.split();
    // We should not forward messages other than text or binary.
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .forward(write)
        .await
        .expect("Failed to forward messages")
}