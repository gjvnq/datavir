#[allow(unused_imports)]
use tokio_tungstenite::connect_async;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use crate::prelude::*;

#[derive(Debug)]
pub struct WSClient {
	addr: String,
	ws_stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>
}

impl Default for WSClient {
	fn default() -> Self {
		WSClient::new(DEFAULT_WS_ADDR)
	}
}

impl WSClient {
	pub fn new(addr: &str) -> WSClient {
		WSClient{
			addr: addr.to_string(),
			ws_stream: None
		}
	}

	pub async fn connect(&mut self) -> DVResult<()> {
		let url = url::Url::parse(&self.addr).unwrap();
		let (ws_stream, _) = match connect_async(url).await {
			Ok((ws_stream, response)) => {
				debug!("ws_stream = {:?}", ws_stream);
				debug!("response = {:?}", response);
				(ws_stream, response)
			},
			Err(err) => {
				error!("Failed to connect to {}: {}", self.addr, err);
				return Err(err)?
			}
		};
		self.ws_stream = Some(ws_stream);
		Ok(())
	}
}