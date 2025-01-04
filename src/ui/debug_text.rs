use egui::Widget;

#[macro_export]
macro_rules! dbgtxt {
    ($debug_text:expr, $($arg:tt)*) => {
        {
            use $crate::ui::debug_text::AddDebugText;
            $debug_text.add_text(format!($($arg)*));
        }
    };
}

#[derive(Clone)]
pub struct DebugText {
	text: Vec<(i32, String)>,
}

impl DebugText {
	pub const fn new() -> Self {
		Self { text: Vec::new() }
	}

	fn add(&mut self, line: impl Into<String>) -> Option<i32> {
		let lowest_priority = *self.text.iter().map(|(p, _)| p).min().unwrap_or(&0);
		self.text.push((lowest_priority - 10, line.into()));
		Some(lowest_priority - 10)
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

pub trait AddDebugText {
	fn add_text(&mut self, line: impl Into<String>) -> Option<i32>;
}

impl AddDebugText for DebugText {
	fn add_text(&mut self, line: impl Into<String>) -> Option<i32> {
		self.add(line)
	}
}

impl AddDebugText for Option<DebugText> {
	fn add_text(&mut self, line: impl Into<String>) -> Option<i32> {
		self.as_mut().and_then(|debug_text| debug_text.add(line))
	}
}

impl AddDebugText for Option<&mut DebugText> {
	fn add_text(&mut self, line: impl Into<String>) -> Option<i32> {
		self.as_mut().and_then(|debug_text| debug_text.add(line))
	}
}
