use crate::api::client::{Error as ApiError, OsmClient};
use crate::map::bounds::Bounds;
use crate::map::map_data::MapData;
use crate::ui::messages::Messages;
use crate::{Element, ElementId, message_error};
use egui::ahash::AHashSet;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

enum DownloadEvent {
	Success { task: DownloadTask, map_data: MapData },
	Error { task: DownloadTask, error: ApiError },
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DownloadTask {
	BBox(Bounds),
	Relation(ElementId),
}

pub struct Downloader {
	client: OsmClient,
	tasks: AHashSet<DownloadTask>,
	map_data_channel: (UnboundedSender<DownloadEvent>, UnboundedReceiver<DownloadEvent>),
	messages: Messages,
}

impl Downloader {
	pub fn new(client: OsmClient, messages: Messages) -> Self {
		Self {
			client,
			tasks: AHashSet::new(),
			map_data_channel: unbounded_channel(),
			messages,
		}
	}

	pub fn is_downloading(&self, task: &DownloadTask) -> bool {
		self.tasks.contains(task)
	}

	pub fn download(&mut self, task: DownloadTask) {
		if !self.tasks.insert(task) {
			return;
		}

		crate::spawn_task!({
			let map_data_sender = self.map_data_channel.0.clone();
			let osm_client = self.client.clone();
			async move {
				let result = match task {
					DownloadTask::BBox(bbox) => osm_client.get_data_from_bounds(bbox).await,
					DownloadTask::Relation(id) => osm_client.get_full(id).await,
				};

				let event = match result {
					Ok(map_data) => DownloadEvent::Success { task, map_data },
					Err(error) => DownloadEvent::Error { task, error },
				};

				let _ = map_data_sender.send(event);
			}
		});
	}

	pub fn apply_changes(&mut self, map_data: &mut MapData) {
		loop {
			match self.map_data_channel.1.try_recv() {
				Ok(DownloadEvent::Success {
					task,
					map_data: new_map_data,
				}) => {
					map_data.join(new_map_data);
					if let DownloadTask::Relation(id) = &task {
						if let Element::Relation { fully_downloaded, .. } =
							&mut map_data.get_mut(id).expect("Relation must exist.").element
						{
							*fully_downloaded = true;
						} else {
							unreachable!("Element must be relation.");
						}
					}
					self.tasks.remove(&task);
				}
				Ok(DownloadEvent::Error { task, error }) => {
					message_error!(self.messages, "Error fetching data:");
					message_error!(self.messages, "{error}");
					self.tasks.remove(&task);
				}
				Err(TryRecvError::Disconnected) => {
					unreachable!("Map data channel can never be disconnected.");
				}
				Err(TryRecvError::Empty) => break,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::map::element::{ElementInfo, ElementType, MapElement};
	use indexmap::IndexMap;

	fn relation_element(id: i64, fully_downloaded: bool) -> MapElement {
		MapElement {
			id: ElementId(id, ElementType::Relation),
			info: ElementInfo::default(),
			element: Element::Relation {
				members: vec![],
				fully_downloaded,
			},
			tags: IndexMap::new(),
			new_tag: None,
			generation: 0,
		}
	}

	fn relation_task(id: i64) -> DownloadTask {
		DownloadTask::Relation(ElementId(id, ElementType::Relation))
	}

	#[test]
	fn apply_changes_clears_task_on_error() {
		let mut downloader = Downloader::new(OsmClient::new(None, None), Messages::new());
		let task = relation_task(1);
		assert!(downloader.tasks.insert(task));
		downloader
			.map_data_channel
			.0
			.send(DownloadEvent::Error {
				task,
				error: ApiError::Xml,
			})
			.expect("Channel send should succeed in test.");

		let mut map_data = MapData::new();
		map_data.add(relation_element(1, false));
		downloader.apply_changes(&mut map_data);

		assert!(!downloader.tasks.contains(&task));
	}

	#[test]
	fn apply_changes_drains_multiple_events_and_updates_relation() {
		let mut downloader = Downloader::new(OsmClient::new(None, None), Messages::new());
		let relation_task = relation_task(10);
		let bbox_task = DownloadTask::BBox(Bounds {
			min: crate::map::latlon::latlon(0.0, 0.0),
			max: crate::map::latlon::latlon(1.0, 1.0),
		});

		assert!(downloader.tasks.insert(relation_task));
		assert!(downloader.tasks.insert(bbox_task));

		let mut relation_data = MapData::new();
		relation_data.add(relation_element(10, false));
		downloader
			.map_data_channel
			.0
			.send(DownloadEvent::Success {
				task: relation_task,
				map_data: relation_data,
			})
			.expect("Channel send should succeed in test.");
		downloader
			.map_data_channel
			.0
			.send(DownloadEvent::Error {
				task: bbox_task,
				error: ApiError::Xml,
			})
			.expect("Channel send should succeed in test.");

		let mut map_data = MapData::new();
		map_data.add(relation_element(10, false));
		downloader.apply_changes(&mut map_data);

		assert!(!downloader.tasks.contains(&relation_task));
		assert!(!downloader.tasks.contains(&bbox_task));

		let relation_id = ElementId(10, ElementType::Relation);
		let relation = map_data.get(&relation_id).expect("Relation must exist.");
		let Element::Relation { fully_downloaded, .. } = &relation.element else {
			panic!("Element must be relation.");
		};
		assert!(*fully_downloaded);
	}
}
