use egui::{Area, Context, Id, InnerResponse, Ui, ViewportBuilder, ViewportId, Window};
use log::info;

/// Returns `None` if the window is not open or native windows is minimized
/// Returns `Some(InnerResponse { inner: None })` if the window is collapsed. (not applicable for native windows)
pub fn show_as_window<R>(
	ctx: &Context,
	title: impl Into<String>,
	open: &mut bool,
	add_contents: impl FnOnce(&mut Ui) -> R + Clone,
) -> Option<InnerResponse<Option<R>>> {
	let title = title.into();
	if ctx.embed_viewports() {
		Window::new(title).open(open).show(ctx, add_contents)
	} else if *open {
		let (response, minimized) = ctx.show_viewport_immediate(
			ViewportId::from_hash_of(&title),
			ViewportBuilder::default().with_title(&title),
			|ctx, _viewport_class| {
				if ctx.input(|i| i.viewport().close_requested()) {
					info!("Hello closing");
					*open = false;
				}
				(
					Area::new(Id::unique(title.clone())).show(ctx, add_contents.clone()),
					ctx.input(|i| i.viewport().minimized.unwrap_or(false)),
				)
			},
		);
		if minimized {
			None
		} else {
			Some(InnerResponse::new(Some(response.inner), response.response))
		}
	} else {
		None
	}
}
