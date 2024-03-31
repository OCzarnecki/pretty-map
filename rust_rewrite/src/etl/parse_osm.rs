use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::str::{self, FromStr};

use log::warn;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use rkyv::ser::{Serializer, serializers::WriteSerializer};
use xz::bufread::XzDecoder;

use crate::{errors, UserConfig};
use crate::data::osm::{OsmId, Node, Way, Relation};
use crate::errors::{Error, Result};
use crate::etl::Etl;

const ETL_NAME: &str = "parse_osm";
const OUTPUT_FILE_NAME: &str = "osm_elements.rkyv";

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct Output {
    // pub nodes: HashMap<OsmId, Node>,
    pub ways: HashMap<OsmId, Way>,
    pub relations: HashMap<OsmId, Relation>,
}

#[derive(Debug, PartialEq)]
enum ParserState {
    Top,
    Node,
    Way,
    Relation,
}

pub struct ParseOsmEtl<'a> {
    config: &'a UserConfig,
    state: ParserState,

    nodes: HashMap<u64, Node>,
    ways: HashMap<u64, Way>,
    relations: HashMap<u64, Relation>,

    current_node: Node,
    current_way: Way,
    current_relation: Relation,
}

impl ParseOsmEtl<'_> {
    fn parse_node(el: &BytesStart) -> Result<Node> {
        let mut id: u64 = 0;
        let mut lat: f64 = 0.0;
        let mut lon: f64 = 0.0;

        for attribute_res in el.attributes() {
            let attribute = attribute_res?;
            match attribute.key.as_ref() {
                b"id" => {
                    let value_str = str::from_utf8(&attribute.value)?;
                    id = value_str.parse()?;
                },
                b"lat" => {
                    let value_str = str::from_utf8(&attribute.value)?;
                    lat = value_str.parse()?;
                },
                b"lon" => {
                    let value_str = str::from_utf8(&attribute.value)?;
                    lon = value_str.parse()?;
                },
                b"version" => (),
                _ => {
                    return Err(format!("WARNING: Unexpected attribute {:?}.", attribute.key).into());
                },
            }
        }

        Ok(Node {
            id,
            lat,
            lon,
            tags: HashMap::new(),
        })
    }

    fn get_attr(el: &BytesStart, attribute_name: &[u8]) -> Result<Vec<u8>> {
        for attribute_res in el.attributes() {
            let attribute = attribute_res?;
            if attribute_name == attribute.key.as_ref() {
                return Ok(attribute.value.to_vec())
            }
        }
        if let Ok(attribute_name_str) = str::from_utf8(attribute_name) {
            Err(format!("Attribute <{:?}> not in element {:?}", attribute_name_str, el).into())
        } else {
            Err(format!("Attribute ??? not in element {:?}", el).into())
        }
    }

    fn parse_attr<T: FromStr>(el: &BytesStart, attribute_name: &[u8]) -> Result<T>
    where errors::Error: From<<T as FromStr>::Err> {
        let attr_value = Self::get_attr(el, attribute_name)?;
        let value_str = str::from_utf8(&attr_value)?;
        let id = value_str.parse()?;
        Ok(id)
    }

    fn parse_way(el: &BytesStart) -> Result<Way> {
        Ok(Way {
            id: Self::parse_attr(el, b"id")?,
            ..Default::default()
        })
    }

    fn parse_relation(el: &BytesStart) -> Result<Relation> {
        Ok(Relation {
            id: Self::parse_attr(el, b"id")?,
            ..Default::default()
        })
    }

    fn create_osm_reader(&self) -> Result<Reader<impl BufRead>> {
        let file = fs::File::open(Path::new("..").join(&self.config.data_path))?;
        let file_reader = BufReader::new(file);
        let xz_reader =  XzDecoder::new(file_reader);
        let buffered_xz_reader = BufReader::new(xz_reader);
        let mut reader = Reader::from_reader(buffered_xz_reader);
        reader.trim_text(true);

        Ok(reader)
    }

    fn start_element(&mut self, e: &BytesStart) -> Result<()> {
        match e.name().as_ref() {
            b"node" => {
                if self.state != ParserState::Top {
                    return Err(format!("Got <node> element in state {:?}", self.state).into())
                }

                self.state = ParserState::Node;
                self.current_node = ParseOsmEtl::parse_node(e)?;
            },
            b"way" => {
                if self.state != ParserState::Top {
                    return Err(format!("Got <way> element in state {:?}", self.state).into())
                }

                self.state = ParserState::Way;
                self.current_way = ParseOsmEtl::parse_way(e)?;
            },
            b"relation" => {
                if self.state != ParserState::Top {
                    return Err(format!("Got <relation> element in state {:?}", self.state).into())
                }

                self.state = ParserState::Relation;
                self.current_relation = ParseOsmEtl::parse_relation(e)?;
            },
            b"nd" => {
                if self.state != ParserState::Way {
                    return Err(format!("Got <nd> element in state {:?}", self.state).into())
                }

                let id = ParseOsmEtl::parse_attr(e, b"ref")?;
                if let Some(node) = self.nodes.get(&id) {
                    self.current_way.nodes.push(node.clone());
                } else {
                    warn!("Reference to undefined node id {:?} while in state {:?}.", id, self.state);
                }
            },
            b"member" => {
                if self.state != ParserState::Relation {
                    return Err(format!("Got <member> element in state {:?}", self.state).into())
                }

                let member_type = ParseOsmEtl::get_attr(e, b"type")?;
                if b"way" != member_type.as_slice() {
                    return Ok(())
                }

                let id = ParseOsmEtl::parse_attr(e, b"ref")?;
                self.current_relation.ways.push(
                    self.ways.get(&id)
                    .ok_or::<Error>(format!("Reference to undefined way id {:?}", id).into())?
                    .clone()
                );
            },
            b"tag" => {
                let key = ParseOsmEtl::get_attr(e, b"k")?.to_vec();
                let value = ParseOsmEtl::get_attr(e, b"v")?.to_vec();

                match self.state {
                    ParserState::Top => return Err(format!("Got unexpected <tag>. {:?}", e).into()),
                    ParserState::Node => self.current_node.tags.insert(key, value),
                    ParserState::Way => self.current_way.tags.insert(key, value),
                    ParserState::Relation => self.current_relation.tags.insert(key, value),
                };
            },
            _ => (),
        }
        Ok(())
    }

    fn end_element(&mut self, name: &[u8]) -> Result<()> {
        match name {
            b"node" => {
                self.state = ParserState::Top;
                self.nodes.insert(self.current_node.id, self.current_node.clone());
            },
            b"way" => {
                self.state = ParserState::Top;
                self.ways.insert(self.current_way.id, self.current_way.clone());
            },
            b"relation" => {
                self.state = ParserState::Top;
                self.relations.insert(self.current_relation.id, self.current_relation.clone());
            },
            _ => {},
        };
        Ok(())
    }

    pub fn new(config: &UserConfig) -> ParseOsmEtl {
        ParseOsmEtl {
            config,
            state: ParserState::Top,

            nodes: HashMap::default(),
            ways: HashMap::default(),
            relations: HashMap::default(),

            current_node: Node::default(),
            current_way: Way::default(),
            current_relation: Relation::default(),
        }
    }
}

impl Etl for ParseOsmEtl<'_> {
    type Input = ();
    type Output = Output;

    fn etl_name(&self) -> &str {
        ETL_NAME
    }

    fn output_file_name(&self) -> &str {
        OUTPUT_FILE_NAME
    }

    fn extract(&mut self) -> Result<Self::Input> {
        Ok(())
    }

    fn transform(&mut self, _input: ()) -> Result<Self::Output> {
        let mut reader = self.create_osm_reader()?;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(e.into()),
                Ok(Event::Eof) => break,
                Ok(Event::Decl(_e)) => (),
                Ok(Event::Text(_e)) => return Err("Didn't expect to see Text in OSM file.".into()),
                Ok(Event::Start(e)) => self.start_element(&e)?,
                Ok(Event::End(e)) => self.end_element(e.name().as_ref())?,
                Ok(Event::Empty(e)) => {
                    self.start_element(&e)?;
                    self.end_element(e.name().as_ref())?;
                },
                event => return Err(format!("Unexpected event {:?}", event).into()),
            }
            buf.clear();
        };
        Ok(Output {
            // nodes: self.nodes.clone(),
            ways: self.ways.clone(),
            relations: self.relations.clone(),
        })
    }

    fn load(&mut self, mut output_file: fs::File, output: Self::Output) -> Result<()> {
        let bytes = rkyv::to_bytes::<_, 256>(&output).unwrap();
        output_file.write_all(&bytes)?;
        Ok(())
    }
}
