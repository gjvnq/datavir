#[allow(unused_imports)]
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use crate::prelude::*;
use tokio::task;

type WSReturn = String;

#[derive(Debug)]
pub struct WSRequestBundle {
	return_ch: mpsc::Sender<WSReturn>,
	data: Vec<u8>,
	close_ws: bool,
}

unsafe impl Send for WSRequestBundle {}
unsafe impl Sync for WSRequestBundle {}

impl WSRequestBundle {
	pub fn new(data: Vec<u8>) -> (Self, mpsc::Receiver<WSReturn>) {
		let (tx, rx) = mpsc::channel();
		return (WSRequestBundle{
			return_ch: tx,
			data: data,
			close_ws: false
		}, rx);
	}

	fn new_close() -> (Self, mpsc::Receiver<WSReturn>) {
		let (tx, rx) = mpsc::channel();
		return (WSRequestBundle{
			return_ch: tx,
			data: Vec::new(),
			close_ws: true
		}, rx);
	}

	pub fn raw_msg(&self) -> RawWsMessage {
		RawWsMessage::Binary(self.data.clone())
	}
}


#[derive(Debug)]
pub struct WSClient {
	addr: String,
	closed: Arc<Mutex<bool>>,
	send_ch: mpsc::SyncSender<WSRequestBundle>,
	_marker: PhantomPinned,
}

unsafe impl Send for WSClient {}

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

		let (tx1, rx1) = mpsc::sync_channel(1);
		let (tx2, rx2) = mpsc::sync_channel(1);
		task::spawn(WSClient::spawn_async(addr.to_string(), tx1.clone(), tx2.clone()));
		match rx1.recv() {
			Ok(v) => if let Some(err) = v {
				return Err(err);
			},
			Err(err) => {
				error!("{:?}", err);
				return Err(err)?;
			}
		};

		let send_ch = match rx2.recv() {
			Ok(v) => v,
			Err(err) => {
				error!("{:?}", err);
				return Err(err)?;
			}
		};

		Ok(WSClient{
			addr: addr.to_string(),
			send_ch: send_ch,
			closed: Arc::new(Mutex::new(false)),
			_marker: PhantomPinned,
		})
	}

	async fn spawn_async(addr: String, tx1: mpsc::SyncSender<Option<DVError>>, tx2: mpsc::SyncSender<mpsc::SyncSender<WSRequestBundle>>) {
		let mut inner = match WSClientInner::new(addr).await {
			Ok(v) => v,
			Err(err) => {
				error!("Failed to create WSClientInner: {:?}", err);
				tx1.send(Some(err)).expect("Failed to send error through channel");
				return;
			}
		};
		tx1.send(None).expect("Failed to send non-error through channel");
		tx2.send(inner.send_ch.clone()).expect("Failed to send send channel through channel");

		inner.run().await;
		return;
	}

	pub async fn ask_time(&self) -> DVResult<String> {
		// 1. Make message
		let raw_msg = "get_time".as_bytes().to_vec();
		let (msg, rx) = WSRequestBundle::new(raw_msg);

		// 3. Send message
		self.send_ch.send(msg)?;

		// 4. Wait for return value
		let ans = rx.recv()?;

		Ok(ans.to_string())
	}

	pub async fn close(&self) -> DVResult<()> {
		info!("Closing WSClient");
		// 1. Make message
		let (msg, rx) = WSRequestBundle::new_close();

		// 3. Send message
		info!("Closing WSClient (Waiting for WSClientInner)");
		self.send_ch.send(msg)?;

		// 4. Wait for return value
		rx.recv()?;

		info!("Closed WSClient");

		{
			let mut closed = self.closed.lock().unwrap();
			*closed = true;
		}

		Ok(())
	}
}

impl Drop for WSClient {
    fn drop(&mut self) {
    	debug!("DROP");
    	let closed = self.closed.lock().unwrap();
    	debug!("closed = {}", *closed);
    	if *closed == false {
        	error!("Forgot to close WSClient");
        }
    }
}


#[derive(Debug)]
struct WSClientInner {
	addr: String,
	recv_ch: mpsc::Receiver<WSRequestBundle>,
	send_ch: mpsc::SyncSender<WSRequestBundle>,
	ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
	_marker: PhantomPinned,
}

impl WSClientInner {
	pub async fn new(addr: String) -> DVResult<WSClientInner> {
		let ws_stream = WSClientInner::make_connection(&addr).await?;
		let (tx, rx) = mpsc::sync_channel(10);
		return Ok(WSClientInner{
			addr: addr.to_string(),
			recv_ch: rx,
			send_ch: tx,
			ws_stream: ws_stream,
			_marker: PhantomPinned,
		})
	}

	pub async fn run(&mut self) {
		// TODO: use select! to listen both on self.recv_ch and self.ws_stream

		// Process messages
		loop {
			// Todo: add better way to stop this loop
			let item = self.recv_ch.recv();
			match item {
				Ok(msg) => {
					info!("got msg = {:?}", msg);
					if msg.close_ws {
						info!("Closing WSClientInner");
						if let Err(err) = self.close().await {
							error!("Failed to close WebSocket: {:?}", err);
						}
						if let Err(err) = msg.return_ch.send("".to_string()) {
							error!("Failed to notify of WebSocket closure: {:?}", err);
						}
						info!("Closed WSClientInner");
						return
					}

					if let Err(err) = self.ws_stream.send(msg.raw_msg()).await {
						error!("Failed to send message through WebSocket: {:?}", err);
					} else {
						let ans = self.ws_stream.next().await;
						if let Err(err) = msg.return_ch.send(ans.unwrap().unwrap().to_string()) {
							error!("Failed to get message from WebSocket: {:?}", err);
						}
					}
				},
				Err(err) => {
					error!("Failed to get msg: {:?}", err);
					break;
				}
			};
		}
	}

	async fn close(&mut self) -> DVResult<()> {
		use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
		use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
		use std::borrow::Cow;
		let close_frame = CloseFrame{
			code: CloseCode::Normal,
			reason: Cow::Owned("Good bye!".to_string()),
		};
		self.ws_stream.close(Some(close_frame)).await?;
		debug!("Closed WebSocket connection to {}", self.addr);

		Ok(())
	}

	async fn make_connection(addr: &str) -> DVResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
		let (ws_stream, _) = match connect_async(addr.to_string()).await {
			Ok(v) => v,
			Err(err) => {
				error!("Failed to connect to {}: {}", addr, err);
				return Err(err)?
			}
		};
		debug!("Opened WebSocket connection to {}", addr);
		return Ok(ws_stream);
	}
}