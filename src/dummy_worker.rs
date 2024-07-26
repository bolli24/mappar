use gloo_worker::Registrable;
fn main() {
	mappar::image_worker::ImageWorker::registrar().register();
}