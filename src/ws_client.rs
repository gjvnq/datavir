#[allow(unused_imports)]
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use crate::prelude::*;

#[derive(Debug)]
pub struct WSClient {
	addr: String,
	// queue: 
	ws_stream: Arc<Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
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
			Ok(v) => v,
			Err(err) => {
				error!("Failed to connect to {}: {}", addr, err);
				return Err(err)?
			}
		};

		Ok(WSClient{
			addr: addr.to_string(),
			ws_stream: Arc::new(Mutex::new(ws_stream)),
			_marker: PhantomPinned,
		})
	}

	pub async fn ask_time(&self) -> DVResult<String> {
		// 1. Make message
		let msg = Message::Text("get_time".to_string());
		// 2. Make return queue

		// 3. Send message

		// 4. Wait for return value
		let ans : Option<Result<Message,tokio_tungstenite::tungstenite::Error>>;
		{
			let mut ws_stream = self.ws_stream.lock().unwrap();
			ws_stream.send(msg).await?;
			ans = ws_stream.next().await;
		}
		let ans = match ans {
			Some(v) => v?,
			None => return Err(DVError::NoMoreResults)
		};

		Ok(ans.to_string())
	}

	pub async fn close(self) -> DVResult<()> {
		use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
		use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
		use std::borrow::Cow;
		let close_frame = CloseFrame{
			code: CloseCode::Normal,
			reason: Cow::Owned("Good bye!".to_string()),
		};
		{
			let mut ws_stream = self.ws_stream.lock().unwrap();
			ws_stream.close(Some(close_frame)).await?;
		}
		Ok(())
	}
}
