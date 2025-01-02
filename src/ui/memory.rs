use egui::util::id_type_map::SerializableAny;
use egui::{Context, Id};

pub trait Memory: SerializableAny + Clone {
	fn load_or_default(ctx: &Context, id: Id) -> Self;
	fn load(ctx: &Context, id: Id) -> Option<Self>;
	fn store(self, ctx: &Context, id: Id);
}
