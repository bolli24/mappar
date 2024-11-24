use egui::Widget;

#[derive(Clone)]
pub struct DebugText {
	text: Vec<(i32, String)>,
}

impl DebugText {
	pub const fn new() -> Self {
		Self { text: Vec::new() }
	}

	pub fn add(&mut self, line: impl Into<String>) -> i32 {
		let lowest_priority = *self.text.iter().map(|(p, _)| p).min().unwrap_or(&0);
		self.text.push((lowest_priority - 10, line.into()));
		lowest_priority - 10
	}

	pub fn add_at(&mut self, line: &str, priority: i32) {
		self.text.push((priority, line.to_owned()));
	}

	pub fn get_string(&self) -> String {
		let mut text = self.text.clone();
		text.sort_by_key(|(p, _)| -p);
		text.into_iter().map(|(_, s)| s).collect::<Vec<String>>().join("\n")
	}

	pub fn clear(&mut self) {
		self.text.clear();
	}
}

impl Widget for DebugText {
	fn ui(self, ui: &mut egui::Ui) -> egui::Response {
		ui.label(egui::RichText::new(self.get_string()).monospace().size(10.0))
	}
}
