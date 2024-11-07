use gloo_worker::Registrable;
fn main() {
	mappar::imagery::image_worker::ImageWorker::registrar().register();
}
