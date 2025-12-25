#[cfg(target_arch = "wasm32")]
use gloo_worker::Registrable;

#[cfg(target_arch = "wasm32")]
fn main() {
	mappar::imagery::image_worker::ImageWorker::registrar().register();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
	panic!("dummy_worker should only be built for wasm32 target");
}
