use crate::imagery::Imageries;
use crate::imagery::imagery_fetcher::Tile;
use crate::ui::messages::Messages;
use alloc::sync::Arc;
use bytes::Bytes;
use epaint::mutex::RwLock;
use indexed_db_futures::Build;
use indexed_db_futures::BuildPrimitive;
use indexed_db_futures::database::Database;
use indexed_db_futures::error::OpenDbError;
use indexed_db_futures::query_source::QuerySource;
use js_sys::Uint8Array;
use log::{error, warn};
use std::collections::HashSet;
use tokio::sync::{Mutex, OnceCell};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::IdbTransactionMode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Unable to open database: {0}")]
	OpenDb(#[from] OpenDbError),

	#[error("Database not initialized")]
	DbeUninit,

	#[error("Database operation error: {0}")]
	DbOperation(#[from] indexed_db_futures::error::Error),

	#[error("Key serialization error: {0}")]
	SerdeJson(#[from] serde_json::Error),

	#[error("Unable to resolve imagery id from tile reference")]
	InvalidImageryRef,

	#[error("Database is missing required object store: {0}")]
	MissingStore(&'static str),

	#[error("Database schema recovery failed after recreate: {0}")]
	SchemaRecovery(Box<Self>),
}

#[derive(Clone)]
pub struct WebCache {
	cached_images: Arc<Mutex<HashSet<String>>>,
	imageries: Arc<RwLock<Imageries>>,
	last_count: usize,
	database: Arc<OnceCell<Database>>,
}

const IMAGERY_STORE: &str = "imagery";
const DB_NAME: &str = "imagery_cache";
const DB_VERSION: u8 = 1;

impl WebCache {
	pub fn new(mut messages: Messages, imageries: Arc<RwLock<Imageries>>) -> Self {
		let database = Arc::new(OnceCell::new());
		let database_clone = database.clone();
		let cached_images = Arc::new(Mutex::new(HashSet::new()));
		let cached_images_clone = cached_images.clone();
		wasm_bindgen_futures::spawn_local(async move {
			match database_clone.get_or_try_init(Self::init_database).await {
				Ok(db) => {
					let t0 = web_time::Instant::now();
					let tiles = Self::load_cached_keys(db).await.unwrap_or_default();
					let count = tiles.len();
					let elapsed = t0.elapsed();
					let mut cache = cached_images_clone.lock().await;
					for tile in tiles {
						cache.insert(tile);
					}
					drop(cache);
					crate::message_info!(
						messages,
						"Found {count} cached images. Took: {:.1}ms",
						elapsed.as_millis()
					);
				}
				Err(e) => {
					error!("Failed to initialize IndexedDB: {e}");
				}
			}
		});

		Self {
			cached_images,
			imageries,
			last_count: 0,
			database,
		}
	}

	fn database(&self) -> Result<Database, Error> {
		self.database.get().cloned().ok_or(Error::DbeUninit)
	}

	async fn database_or_init(&self) -> Result<Database, Error> {
		log::info!("database_or_init");
		if let Some(db) = self.database.get() {
			log::info!("got it");
			return Ok(db.clone());
		}

		let db = self.database.get_or_try_init(Self::init_database).await?;
		log::info!("init done");
		Ok(db.clone())
	}

	fn tile_key(&self, tile: &Tile) -> Result<String, Error> {
		let imageries = self.imageries.read();
		let imagery_id = imageries
			.get(tile.imagery_ref)
			.map(|imagery| imagery.id.0.clone())
			.ok_or(Error::InvalidImageryRef)?;
		let (x, y, z) = tile.coords;
		Ok(serde_json::to_string(&(imagery_id, z, x, y))?)
	}

	async fn load_cached_keys(db: &Database) -> Option<Vec<String>> {
		let transaction = db.transaction(IMAGERY_STORE).build().ok()?;
		let store = transaction.object_store(IMAGERY_STORE).ok()?;
		let fut = store.get_all_keys::<String>().primitive().ok()?;
		let keys = fut.await.ok()?;
		Some(keys.filter_map(Result::ok).collect())
	}

	async fn init_database() -> Result<Database, Error> {
		log::info!("initializing");

		let db = Self::open_database().await?;
		if Self::has_object_store(&db, IMAGERY_STORE) {
			log::info!("initialized");
			return Ok(db);
		}

		warn!("IndexedDB is missing object store `{IMAGERY_STORE}`. Recreating database `{DB_NAME}`",);
		db.close();
		Database::delete_by_name(DB_NAME)?.await?;

		let recreated = Self::open_database().await?;
		if Self::has_object_store(&recreated, IMAGERY_STORE) {
			warn!("IndexedDB `{}` recreated successfully", DB_NAME);
			log::info!("initialized");
			return Ok(recreated);
		}

		error!(
			"IndexedDB recovery failed: database `{DB_NAME}` still missing object store `{IMAGERY_STORE}` after recreate"
		);
		recreated.close();
		Err(Error::SchemaRecovery(Box::new(Error::MissingStore(IMAGERY_STORE))))
	}

	async fn open_database() -> Result<Database, Error> {
		Database::open(DB_NAME)
			.with_version(DB_VERSION)
			.with_on_upgrade_needed(|event, db| {
				let old_version = event.old_version() as u64;
				if old_version == 0 || !Self::has_object_store(&db, IMAGERY_STORE) {
					db.create_object_store(IMAGERY_STORE)
						.with_auto_increment(true)
						.build()?;
				}
				Ok(())
			})
			.await
			.map_err(Into::into)
	}

	fn has_object_store(db: &Database, store_name: &str) -> bool {
		db.object_store_names().any(|name| name == store_name)
	}

	pub fn cached_count(&mut self) -> usize {
		self.last_count = self.cached_images.try_lock().map_or(self.last_count, |h| h.len());
		self.last_count
	}

	pub async fn save_image(&self, tile: Tile, image_data: &Bytes) -> Result<(), Error> {
		let db = self.database_or_init().await?;
		log::info!("Store tile {tile:?}");

		let transaction = db
			.transaction(IMAGERY_STORE)
			.with_mode(IdbTransactionMode::Readwrite)
			.build()?;

		let store = transaction.object_store(IMAGERY_STORE)?;

		let tile_key = self.tile_key(&tile)?;
		let array = Uint8Array::new_with_length(image_data.len() as u32);
		array.copy_from(image_data.as_ref());
		let js_val: JsValue = array.into();

		store.put(&js_val).with_key(tile_key.as_str()).primitive()?.await?;

		transaction.commit().await?;

		self.cached_images.lock().await.insert(tile_key);

		Ok(())
	}

	pub async fn get_image(&self, tile: Tile) -> Option<Bytes> {
		let db = self.database_or_init().await.ok()?;

		let tile_key = self.tile_key(&tile).ok()?;

		let transaction = db
			.transaction(IMAGERY_STORE)
			.with_mode(IdbTransactionMode::Readonly)
			.build()
			.ok()?;

		let store = transaction.object_store(IMAGERY_STORE).ok()?;

		let js_val: Option<JsValue> = store.get(tile_key).primitive().ok()?.await.ok()?;

		let array = js_val?.dyn_into::<Uint8Array>().ok()?;
		Some(Bytes::from(array.to_vec()))
	}
}

impl Drop for WebCache {
	fn drop(&mut self) {
		if Arc::strong_count(&self.database) == 1
			&& let Some(db) = self.database.get().cloned()
		{
			db.close();
		}
	}
}
