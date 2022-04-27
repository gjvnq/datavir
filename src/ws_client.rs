#[allow(unused_imports)]
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;
use crate::prelude::*;

type WSReturn = String;

#[derive(Debug)]
pub struct WSRequest {
	tx: mpsc::Sender<WSReturn>,
	raw: Message
}

impl WSRequest {
	pub fn from_raw(raw: Message) -> (Self, mpsc::Receiver<WSReturn>) {
		let (tx, rx) = mpsc::channel();
		return (WSRequest{
			tx: tx,
			raw: raw
		}, rx);
	}
}


#[derive(Debug)]
pub struct WSClient {
	addr: String,
	send_ch: Option<mpsc::Sender<WSRequest>>,
	_marker: PhantomPinned,
}

unsafe impl Send for WSClient {
	// add code here
}

impl WSClient {
	#[allow(dead_code)]
	pub fn get_addr(&self) -> String {
		self.addr.clone()
	}

	pub async fn new(addr: &str) -> DVResult<WSClient> {
		let url = url::Url::parse(addr);
		if url.is_err() {
			return Err(DVError::InvalidUrl(addr.to_string()))
		}

		Ok(WSClient{
			addr: addr.to_string(),
			send_ch: None,
			_marker: PhantomPinned,
		})
	}

	pub async fn main_loop(&mut self) -> DVResult<()> {
		if self.send_ch.is_some() {
			return Ok(());
		}

		// Make WebSocket connection
		let (mut ws_stream, _) = match connect_async(self.addr.clone()).await {
			Ok(v) => v,
			Err(err) => {
				error!("Failed to connect to {}: {}", self.addr, err);
				return Err(err)?
			}
		};

		// Prepare
		let (tx, rx) = mpsc::channel();
		self.send_ch = Some(tx);

		// Process messages
		loop {
			// Todo: add better way to stop this loop
			match rx.recv() {
				Ok(msg) => {
					info!("got msg = {:?}", msg);
					ws_stream.send(msg.raw).await?;
					let ans = ws_stream.next().await;
					msg.tx.send(ans.unwrap().unwrap().to_string())?;
				},
				Err(err) => {
					error!("Failed to get msg: {:?}", err);
					break;
				}
			};
		}

		// Close gracefully
		use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
		use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
		use std::borrow::Cow;
		let close_frame = CloseFrame{
			code: CloseCode::Normal,
			reason: Cow::Owned("Good bye!".to_string()),
		};
		ws_stream.close(Some(close_frame)).await?;

		Ok(())
	}

	pub async fn ask_time(&self) -> DVResult<String> {
		let send_ch = match &self.send_ch {
			Some(v) => v,
			None => return Err(DVError::NotReady("main_loop not running".to_string()))
		};

		// 1. Make message
		let raw_msg = Message::Text("get_time".to_string());
		let (msg, rx) = WSRequest::from_raw(raw_msg);

		// 3. Send message
		send_ch.send(msg)?;

		// 4. Wait for return value
		let ans = rx.recv()?;

		Ok(ans.to_string())
	}
}
