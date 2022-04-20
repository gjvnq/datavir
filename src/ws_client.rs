#[allow(unused_imports)]
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use crate::prelude::*;

#[derive(Debug)]
pub struct WSClient {
	addr: String,
	ops: i32,
	ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
	_marker: PhantomPinned,
}

impl WSClient {
	#[allow(dead_code)]
	pub fn get_addr(&self) -> String {
		self.addr.clone()
	}

	pub async fn new(addr: &str) -> DVResult<WSClient> {
		let url = url::Url::parse(addr).unwrap();
		let (ws_stream, _) = match connect_async(url).await {
			Ok((ws_stream, response)) => {
				debug!("ws_stream = {:?}", ws_stream);
				debug!("response = {:?}", response);
				(ws_stream, response)
			},
			Err(err) => {
				error!("Failed to connect to {}: {}", addr, err);
				return Err(err)?
			}
		};

		Ok(WSClient{
			addr: addr.to_string(),
			ops: 0,
			ws_stream: ws_stream,
			_marker: PhantomPinned,
		})
	}

	pub async fn ask_time(&mut self) -> DVResult<String> {
		self.ops += 1;
		info!("ops = {}", self.ops);
		self.ws_stream.send(Message::Text("get_time".to_string())).await?;
		Ok("err".to_string())
	}

	pub async fn close(mut self) -> DVResult<()> {
		use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
		use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
		use std::borrow::Cow;
		let close_frame = CloseFrame{
			code: CloseCode::Normal,
			reason: Cow::Owned("Good bye!".to_string()),
		};
		self.ws_stream.close(Some(close_frame)).await?;
		Ok(())
	}
}
