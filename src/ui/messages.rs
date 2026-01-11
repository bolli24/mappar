use crate::util::history::History;
use egui::{Frame, Style, Widget};
use epaint::mutex::Mutex;
use std::sync::Arc;
use web_time::Instant;

#[derive(Clone)]
pub struct Messages {
	messages: Arc<Mutex<History<String>>>,
	startup: Instant,
}

const MAX_MESSAGE_COUNT: usize = 100;
const MAX_MESSAGE_AGE: f32 = 5.0;

impl Messages {
	#[must_use]
	pub fn new() -> Self {
		Self {
			messages: Arc::new(Mutex::new(History::new(0..MAX_MESSAGE_COUNT, MAX_MESSAGE_AGE))),
			startup: Instant::now(),
		}
	}

	pub fn add(&mut self, message: &(impl ToString + ?Sized)) {
		self.messages.lock().add(self.now(), message.to_string());
	}
	#[must_use]
	pub fn now(&self) -> f64 {
		self.startup.elapsed().as_secs_f64()
	}
}

impl Widget for Messages {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		let mut messages = self.messages.lock();
		messages.flush(self.now());
		let mut style: Style = ui.style().as_ref().clone();
		style.visuals.window_fill = style.visuals.window_fill.gamma_multiply(0.80);
		ui.vertical(|ui| {
			ui.style_mut().spacing.item_spacing.y = 4.0;
			for message in messages.values() {
				Frame::popup(&style).show(ui, |ui| ui.label(message));
			}
		})
		.response
	}
}
