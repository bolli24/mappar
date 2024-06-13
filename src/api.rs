use std::fmt::{Debug, Display};
use std::str::from_utf8;

use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::StatusCode;

use crate::map_data::{
    Bounds, Element, ElementInfo, ElementType, LatLon, MapData, MapElement, Member, Tag,
};

const API_URL: &str = "https://api.openstreetmap.org";
const API_VERSION: &str = "0.6";

const API_BOUNDING_BOX: &str = "/api/0.6/map?bbox=";

pub enum Error {
    Reqwest(reqwest::Error),
    ApiError(StatusCode, String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Reqwest(error) => write!(f, "Reqwest error: {}", error),
            Error::ApiError(status, body) => write!(f, "API error: {} - {}", status, body),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Reqwest(error)
    }
}

pub async fn get_data(bounds: Bounds) -> Result<MapData, Error> {
    println!("Getting data from the API");
    let response = reqwest::get(format!(
        "{}{}{}",
        API_URL,
        API_BOUNDING_BOX,
        bounds.as_string()
    ))
    .await?;

    if !response.status().is_success() {
        println!("Failed to get data from the API");
        return Err(Error::ApiError(response.status(), response.text().await?));
    }

    println!("Response: {:?}", response);
    println!("status: {:?}", response.status());

    let xml = response.text().await?;

    let mut data = MapData::new();
    let mut reader = Reader::from_str(&xml);

    loop {
        match reader.read_event() {
            Ok(event) => match event {
                Event::Start(ref e) => {
                    let attributes = e.attributes();

                    let mut tags: Option<Vec<Tag>> = None;
                    let mut nodes = Vec::new();
                    let mut members = Vec::new();
                    loop {
                        match reader.read_event() {
                            Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                                b"tag" => {
                                    let mut key = String::new();
                                    let mut value = String::new();

                                    for a in e.attributes() {
                                        let a = a.unwrap();
                                        match a.key.as_ref() {
                                            b"k" => {
                                                key = from_utf8(&a.value).unwrap().to_string();
                                            }
                                            b"v" => {
                                                value = from_utf8(&a.value).unwrap().to_string();
                                            }
                                            _ => {}
                                        }
                                    }
                                    if !key.is_empty() && !value.is_empty() {
                                        tags.get_or_insert(Vec::new()).push(Tag {
                                            key: key.to_string(),
                                            value: value.to_string(),
                                        });
                                    }
                                }
                                b"nd" => {
                                    let node_ref = e.attributes().next().unwrap().unwrap();
                                    if node_ref.key.as_ref() == b"ref" {
                                        nodes.push(
                                            from_utf8(&node_ref.value).unwrap().parse().unwrap(),
                                        );
                                    }
                                }
                                b"member" => {
                                    let mut ref_id: i64 = -1;
                                    let mut role = "".to_string();
                                    let mut member_type = ElementType::Node;
                                    for attr in e.attributes() {
                                        let attr = attr.unwrap();
                                        match attr.key.as_ref() {
                                            b"ref" => {
                                                ref_id = from_utf8(attr.value.as_ref())
                                                    .unwrap()
                                                    .parse()
                                                    .unwrap();
                                            }
                                            b"role" => {
                                                role = from_utf8(attr.value.as_ref())
                                                    .unwrap()
                                                    .to_string();
                                            }
                                            b"type" => {
                                                member_type = ElementType::from_str(
                                                    from_utf8(attr.value.as_ref()).unwrap(),
                                                )
                                                .unwrap()
                                            }
                                            _ => {
                                                eprintln!("Unknown attribute on relation member: {:?}=\"{:?}\"", attr.key, attr.value)
                                            }
                                        }
                                    }
                                    members.push(Member {
                                        ref_id,
                                        member_type,
                                        role,
                                    });
                                }
                                _ => {}
                            },
                            Ok(Event::End(_)) => {
                                break;
                            }
                            _ => {}
                        }
                    }

                    if !matches!(e.name().as_ref(), b"node" | b"way" | b"relation") {
                        eprintln!("Unknown element: {:?}", e);
                        continue;
                    }

                    let mut lat = f64::NAN;
                    let mut lon = f64::NAN;
                    let mut id = 0;
                    let mut info = ElementInfo::default();

                    for attr in attributes {
                        let attr = attr.unwrap();
                        match attr.key.as_ref() {
                            b"lat" => {
                                lat = from_utf8(&attr.value).unwrap().parse().unwrap();
                            }
                            b"lon" => {
                                lon = from_utf8(&attr.value).unwrap().parse().unwrap();
                            }
                            b"id" => {
                                id = from_utf8(&attr.value).unwrap().parse().unwrap();
                            }
                            b"version" => {
                                info.version = from_utf8(&attr.value).unwrap().parse().unwrap();
                            }
                            b"changeset" => {
                                info.changeset = from_utf8(&attr.value).unwrap().parse().unwrap();
                            }
                            b"uid" => {
                                info.uid = from_utf8(&attr.value).unwrap().parse().ok();
                            }
                            b"user" => {
                                info.user = Some(from_utf8(&attr.value).unwrap().to_string());
                            }
                            b"visible" => {
                                info.visible = from_utf8(&attr.value).unwrap() == "true";
                            }
                            b"timestamp" => {
                                info.timestamp = from_utf8(&attr.value).unwrap().parse().unwrap();
                            }
                            _ => {
                                eprintln!("Unknown attribute: {:?}", attr);
                            }
                        }
                    }

                    let element = match e.name().as_ref() {
                        b"node" => Element::Node {
                            pos: LatLon(lat, lon),
                        },
                        b"way" => Element::Way { nodes },
                        b"relation" => Element::Relation { members },
                        _ => {
                            eprintln!("Unknown element type: {:?}", e);
                            continue;
                        }
                    };
                    data.add(MapElement {
                        info,
                        element,
                        tags,
                        id,
                    });
                }

                Event::Eof => {
                    println!("EOF");
                    break;
                }
                _ => {}
            },
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use crate::map_data::LatLon;

    use super::*;

    #[tokio::test]
    async fn test_get_data() {
        let bounds = Bounds {
            min: LatLon(52.17562, 11.63463),
            max: LatLon(52.17039, 11.64465),
        };
        let result = get_data(bounds).await;
        println!("Result: {:?}", result);
    }
}
