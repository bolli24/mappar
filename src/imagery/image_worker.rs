use crate::imagery::imagery_fetcher::Tile;
use bytes::Bytes;
use gloo_worker::{HandlerId, Worker, WorkerScope};
use image::ImageError;
use log::{error, info};
use std::io::Cursor;

#[derive(Debug)]
pub struct Message(pub u32);
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Input(pub Tile, pub Vec<u8>);
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Output(pub Tile, pub Option<Vec<u8>>);

// TODO: use ImageDecoder once ready on all browsers
// TODO MAYBE: fetch images in web worker.
pub struct ImageWorker {}

impl Worker for ImageWorker {
	type Message = Message;
	type Input = Input;
	type Output = Output;

	fn create(_scope: &WorkerScope<Self>) -> Self {
		info!("Image Worker created");
		Self {}
	}

	fn update(&mut self, _scope: &WorkerScope<Self>, msg: Self::Message) {
		info!("Image Worker received message: {:?}", msg);
	}

	fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
		let bytes = Bytes::from(msg.1);
		let decoded = match decode(bytes) {
			Ok(decoded) => Some(decoded),
			Err(err) => {
				error!("Failed to decode image: {:?}", err);
				None
			}
		};

		info!("Image Worker decoded image");

		scope.respond(id, Output(msg.0, decoded));
	}
}

fn decode(bytes: Bytes) -> Result<Vec<u8>, ImageError> {
	let decoded = image::ImageReader::new(Cursor::new(bytes))
		.with_guessed_format()?
		.decode()?
		.to_rgba8()
		.to_vec();

	Ok(decoded)
}
