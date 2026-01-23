use crate::api::auth::{OsmAuthToken, login};
use crate::api::client::OsmClient;
use crate::ui::memory::TempMemory;
use crate::ui::messages::Messages;
use crate::{message_error, message_info, spawn_task};
use egui::{Id, Response, Ui, Widget};
use mappar_macros::TempMemory;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

async fn fetch_user_info(osm_client: Arc<tokio::sync::Mutex<OsmClient>>, messages: &mut Messages) {
	let mut client_guard = osm_client.lock().await;
	let user_details = client_guard.get_user_details().await;
	match user_details {
		Ok(user_details) => {
			client_guard.user = Some(user_details);
		}
		Err(error) => {
			message_error!(messages, "Error getting user details: {:?}", error);
			client_guard.auth = None;
		}
	}
}

#[derive(derive_new::new)]
pub struct AuthWidget {
	osm_client: Arc<tokio::sync::Mutex<OsmClient>>,
	messages: Messages,
}

#[derive(Clone, TempMemory)]
pub struct AuthMemory {
	pub tried_getting_token: bool,
	pub cancellation: CancellationToken,
}

impl Default for AuthMemory {
	fn default() -> Self {
		let token = CancellationToken::new();
		token.cancel();
		Self {
			tried_getting_token: false,
			cancellation: token,
		}
	}
}

impl Widget for AuthWidget {
	fn ui(mut self, ui: &mut Ui) -> Response {
		let id = Id::from("auth_widget");
		let mut mem: AuthMemory = TempMemory::load_or_default(ui.ctx(), id);
		let mut logout = false;

		let Ok(mut client_guard) = self.osm_client.try_lock() else {
			return ui.spinner();
		};

		if !mem.cancellation.is_cancelled() && client_guard.auth.is_none() {
			if ui.button("Cancel").clicked() {
				mem.cancellation.cancel();
			} else {
				return ui.spinner();
			}
		}

		let osm_client = self.osm_client.clone();

		let response = if let Some(user_details) = &client_guard.user {
			ui.horizontal(|ui| {
				ui.label(&user_details.display_name)
					.on_hover_text(user_details.id.to_string());
				logout = ui.button("Logout").clicked();
			})
			.response
		} else {
			if !mem.tried_getting_token {
				if let Some(token) = get_token(&client_guard, &mut self.messages) {
					log::info!("Got login token: {}...", &token.0[0..5]);
					client_guard.auth = Some(token);
					let osm_client = self.osm_client.clone();
					let mut messages = self.messages.clone();
					spawn_task!(async move {
						fetch_user_info(osm_client, &mut messages).await;
					});
				} else {
					log::warn!("Could not get login token");
				}
				mem.tried_getting_token = true;
			}

			let login_btn = ui.button("Login to Osm");
			if login_btn.clicked() {
				let mut messages = self.messages.clone();
				let cancellation = CancellationToken::new();
				mem.cancellation = cancellation.clone();

				spawn_task!(async move {
					match tokio::select! {
						result = login() => result,
						_ = cancellation.cancelled() => Err(anyhow::anyhow!("Login flow canceled by user."))
					} {
						Ok(token) => {
							let mut client_guard = osm_client.lock().await;
							store_token(&mut client_guard, &token, &mut messages);
							client_guard.auth = Some(OsmAuthToken(token));
							message_info!(messages, "Locked in successfully.");
							drop(client_guard);
							fetch_user_info(osm_client, &mut messages).await;
						}
						Err(err) => {
							message_error!(messages, "Error authenticating user: {err}");
						}
					}
					cancellation.cancel();
				});
			}
			login_btn
		};

		if logout {
			client_guard.auth = None;
			client_guard.user = None;
			delete_stored_token(&client_guard, &mut self.messages);
		}

		mem.store(ui.ctx(), id);

		response
	}
}

#[cfg(target_arch = "wasm32")]
fn token_item() -> String {
	format!("token_{}", crate::api::OAUTH_URL)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_token(client: &OsmClient, messages: &mut Messages) -> Option<OsmAuthToken> {
	match client.credential_entry.get_password() {
		Err(keyring::Error::NoEntry) => None,
		Err(err) => {
			message_error!(messages, "Error retrieving token from keyring: {err}");
			let _ = client
				.credential_entry
				.delete_credential()
				.inspect_err(|err| match err {
					keyring::Error::NoEntry => (),
					err => message_error!(messages, "Error deleting token from keyring: {err}"),
				});
			None
		}
		Ok(token) => Some(OsmAuthToken(token)),
	}
}

#[cfg(target_arch = "wasm32")]
fn get_token(_client: &OsmClient, _messages: &mut Messages) -> Option<OsmAuthToken> {
	let window = web_sys::window().expect("Window must be available.");
	let storage = window
		.local_storage()
		.expect("Local storage must be available.")
		.expect("Local storage must be available.");
	storage
		.get_item(&token_item())
		.expect("Get_item must not error.")
		.map(OsmAuthToken)
}

// TODO: message error
#[cfg(not(target_arch = "wasm32"))]
fn store_token(client: &mut OsmClient, token: &str, messages: &mut Messages) {
	let _ = client.credential_entry.set_password(token).inspect_err(|err| {
		message_error!(messages, "Error storing token: {:?}", err);
	});
}

#[cfg(target_arch = "wasm32")]
fn store_token(_client: &mut OsmClient, token: &str, messages: &mut Messages) {
	let window = web_sys::window().expect("Window must be available.");
	let storage = window
		.local_storage()
		.expect("Local storage must be available.")
		.expect("Local storage must be available.");
	let _ = storage.set_item(&token_item(), token).inspect_err(|err| {
		message_error!(messages, "Error storing token: {:?}", err);
	});
}

#[cfg(not(target_arch = "wasm32"))]
fn delete_stored_token(client: &OsmClient, messages: &mut Messages) {
	match client.credential_entry.delete_credential() {
		Err(keyring::Error::NoEntry) => (),
		Err(err) => message_error!(messages, "Error deleting token: {:?}", err),
		_ => (),
	}
}

#[cfg(target_arch = "wasm32")]
fn delete_stored_token(_client: &OsmClient, _messages: &mut Messages) {
	let window = web_sys::window().expect("Window must be available.");
	let storage = window
		.local_storage()
		.expect("Local storage must be available.")
		.expect("Local storage must be available.");
	let _ = storage.delete(&token_item()).expect("Delete must not error.");
}
