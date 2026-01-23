use crate::api::{OAUTH_URL, USER_AGENT};
use dotenvy_macro::dotenv;
use oauth2::basic::BasicClient;
use oauth2::reqwest;
use oauth2::{
	AuthUrl, AuthorizationCode, ClientId, CsrfToken, EndpointNotSet, EndpointSet, PkceCodeChallenge, RedirectUrl,
	Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use url::Url;

#[derive(Clone)]
pub struct OsmAuthToken(pub String);

type OsmClient = BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[cfg(not(target_arch = "wasm32"))]
fn create_client(port: u16) -> OsmClient {
	BasicClient::new(ClientId::new(dotenv!("OSM_CLIENT_ID").to_owned()))
		.set_auth_uri(AuthUrl::new(format!("{OAUTH_URL}/oauth2/authorize")).unwrap())
		.set_token_uri(TokenUrl::new(format!("{OAUTH_URL}/oauth2/token")).unwrap())
		.set_redirect_uri(RedirectUrl::new(format!("http://127.0.0.1:{}/callback", port)).unwrap())
}

#[cfg(target_arch = "wasm32")]
fn create_client(origin: String) -> OsmClient {
	BasicClient::new(ClientId::new(dotenv!("OSM_CLIENT_ID").to_owned()))
		.set_auth_uri(AuthUrl::new(format!("{OAUTH_URL}/oauth2/authorize")).unwrap())
		.set_token_uri(TokenUrl::new(format!("{OAUTH_URL}/oauth2/token")).unwrap())
		.set_redirect_uri(RedirectUrl::new(format!("{origin}/login.html")).unwrap())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn login() -> anyhow::Result<String> {
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
	use tokio::net::TcpListener;

	let mut listener_result = None;

	for port in [8080, 8081, 8082, 8083, 8084] {
		if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{port}")).await {
			listener_result = Some((listener, port));
			break;
		}
	}

	let (listener, port) = listener_result.ok_or_else(|| anyhow::anyhow!("Failed to bind to ports 8080-8084"))?;

	let client = create_client(port);
	let http_client = reqwest::ClientBuilder::new()
		.user_agent(USER_AGENT)
		.redirect(reqwest::redirect::Policy::none())
		.build()?;

	let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

	let (auth_url, csrf_token) = get_auth_url(&client, pkce_challenge);

	open::that(auth_url.to_string())?;

	let mut stream = match listener.accept().await {
		Ok((s, _)) => s,
		Err(e) => {
			return Err(anyhow::anyhow!("Error accepting connection: {e}"));
		}
	};

	let mut reader = BufReader::new(&mut stream);
	let mut request_line = String::new();
	reader.read_line(&mut request_line).await?;

	let mut headers = [httparse::EMPTY_HEADER; 16];
	let mut req = httparse::Request::new(&mut headers);
	let _ = req.parse(request_line.as_bytes())?;
	let path = req.path.ok_or_else(|| anyhow::anyhow!("No path variable"))?;
	let parsed_url = Url::parse(&format!("https://localhost{path}"))?;

	let code = parsed_url
		.query_pairs()
		.find(|(key, _)| key == "code")
		.map(|(_, val)| AuthorizationCode::new(val.into_owned()))
		.ok_or_else(|| anyhow::anyhow!("authorization code missing"))?;

	let callback_state = parsed_url
		.query_pairs()
		.find(|(key, _)| key == "state")
		.map(|(_, val)| val.into_owned())
		.ok_or_else(|| anyhow::anyhow!("state parameter missing"))?;

	if callback_state != csrf_token.secret().as_str() {
		return Err(anyhow::anyhow!("CSRF token mismatch"));
	}

	let token_response = match client
		.exchange_code(AuthorizationCode::new(code.secret().clone()))
		.set_pkce_verifier(pkce_verifier)
		.request_async(&http_client)
		.await
	{
		Ok(response) => response,
		Err(e) => {
			let response = "HTTP/1.1 400 Bad Request\r\n\r\n<html><h2>Authentication failed. Check the app logs for details.</h2></html>";
			let _ = stream.write_all(response.as_bytes()).await;
			return Err(anyhow::anyhow!("Token exchange failed: {e}"));
		}
	};

	let access_token = token_response.access_token().secret().clone();

	let response = "HTTP/1.1 200 OK\r\n\r\n<html><h2>Authentication successful! You can close this tab.</h2></html>";

	if let Err(e) = stream.write_all(response.as_bytes()).await {
		log::warn!("Failed to send response to browser: {e}");
	}

	Ok(access_token)
}

#[cfg(target_arch = "wasm32")]
pub async fn login() -> anyhow::Result<String> {
	use wasm_bindgen::prelude::*;
	use web_sys::{BroadcastChannel, MessageEvent, window};

	#[derive(Deserialize)]
	struct OAuthCallbackMessage {
		code: String,
		state: String,
	}

	let client = create_client(crate::util::get_current_url_origin());

	let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

	let (auth_url, csrf_token) = get_auth_url(&client, pkce_challenge);

	let window = window().expect("Window must exist.");
	let channel = BroadcastChannel::new("osm_oauth").expect("Can't open broadcast channel.");

	let (sender, mut receiver) = tokio::sync::mpsc::channel::<(String, String)>(1);
	let closure = Closure::wrap(Box::new({
		let sender = sender.clone();
		move |event: MessageEvent| {
			let Some(payload) = event.data().as_string() else {
				log::warn!("OAuth callback message must be a string");
				let _ = sender.blocking_send((String::new(), String::new()));
				return;
			};

			match serde_json::from_str::<OAuthCallbackMessage>(&payload) {
				Ok(msg) => {
					let _ = sender.blocking_send((msg.code, msg.state));
				}
				Err(err) => {
					log::warn!("Failed to parse OAuth callback message: {err}");
					let _ = sender.blocking_send((String::new(), String::new()));
				}
			}
		}
	}) as Box<dyn FnMut(MessageEvent)>);

	channel.set_onmessage(Some(closure.as_ref().unchecked_ref()));

	window
		.open_with_url_and_target_and_features(auth_url.as_str(), "_blank", "width=500,height=500,popup=yes")
		.map_err(|_| anyhow::anyhow!("Unable to open popup"))?;

	let (code, state) = receiver.recv().await.expect("Channel must not be closed.");
	channel.close();

	if state != csrf_token.secret().as_str() {
		return Err(anyhow::anyhow!("CSRF token mismatch"));
	}

	let http_client = reqwest::ClientBuilder::new().user_agent(USER_AGENT).build()?;

	let token_response = match client
		.exchange_code(AuthorizationCode::new(code))
		.set_pkce_verifier(pkce_verifier)
		.request_async(&http_client)
		.await
	{
		Ok(response) => response,
		Err(e) => {
			return Err(anyhow::anyhow!("Token exchange failed: {e}"));
		}
	};

	let access_token = token_response.access_token().secret().clone();

	Ok(access_token)
}

fn get_auth_url(client: &OsmClient, pkce_challenge: PkceCodeChallenge) -> (Url, CsrfToken) {
	client
		.authorize_url(CsrfToken::new_random)
		.add_scope(Scope::new("read_prefs".to_string()))
		.add_scope(Scope::new("write_api".to_string()))
		.set_pkce_challenge(pkce_challenge)
		.url()
}
