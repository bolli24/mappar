use js_sys::Promise;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(module = "/module.js")]
extern "C" {
	fn sleep(ms: i32) -> Promise;
}

pub async fn timer(ms: i32) -> Result<(), JsValue> {
	let promise = sleep(ms);
	let js_fut = JsFuture::from(promise);
	js_fut.await?;
	Ok(())
}
