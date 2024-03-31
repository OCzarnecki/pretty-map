use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::str::{self, FromStr};

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use xz::bufread::XzDecoder;

use crate::{errors, UserConfig};
use crate::data::osm::{OsmId, Node, Way, Relation};
use crate::errors::{Error, Result};
use crate::etl::Etl;

const ETL_NAME: &str = "parse_osm";
const OUTPUT_FILE_NAME: &str = "osm_elements.rkyv";

pub struct Output {
    nodes: HashMap<OsmId, Node>,
    ways: HashMap<OsmId, Way>,
    relations: HashMap<OsmId, Relation>,
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
}

impl ParseOsmEtl<'_> {
    fn parse_node(el: BytesStart) -> Result<Node> {
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
        Err(format!("Attribute 'id' not in element {:?}", el).into())
    }

    fn parse_attr<T: FromStr>(el: BytesStart, attribute_name: &[u8]) -> Result<T>
    where errors::Error: From<<T as FromStr>::Err> {
        let attr_value = Self::get_attr(&el, attribute_name)?;
        let value_str = str::from_utf8(&attr_value)?;
        let id = value_str.parse()?;
        Ok(id)
    }

    fn parse_way(el: BytesStart) -> Result<Way> {
        Ok(Way {
            id: Self::parse_attr(el, b"id")?,
            ..Default::default()
        })
    }

    fn parse_relation(el: BytesStart) -> Result<Relation> {
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

    pub fn new(config: &UserConfig) -> ParseOsmEtl {
        ParseOsmEtl {
            config
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

    fn extract(&self) -> Result<Self::Input> {
        Ok(())
    }

    fn transform(&self, _input: ()) -> Result<Self::Output> {
        let mut reader = self.create_osm_reader()?;
        let mut buf = Vec::new();

        let mut state = ParserState::Top;

        let mut nodes: HashMap<u64, Node> = HashMap::new();
        let mut ways: HashMap<u64, Way> = HashMap::new();
        let mut relations: HashMap<u64, Relation> = HashMap::new();

        let mut current_node = Node::default();
        let mut current_way = Way::default();
        let mut current_relation = Relation::default();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(e.into()),
                Ok(Event::Eof) => break,
                Ok(Event::Decl(_e)) => (),
                Ok(Event::Text(_e)) => return Err("Didn't expect to see Text in OSM file.".into()),
                Ok(Event::Start(e)) => {
                    match e.name().as_ref() {
                        b"node" => {
                            if state != ParserState::Top {
                                return Err(format!("Got <node> element in state {:?}", state).into())
                            }

                            state = ParserState::Node;
                            current_node = ParseOsmEtl::parse_node(e)?;
                        },
                        b"way" => {
                            if state != ParserState::Top {
                                return Err(format!("Got <way> element in state {:?}", state).into())
                            }

                            state = ParserState::Way;
                            current_way = ParseOsmEtl::parse_way(e)?;
                        },
                        b"relation" => {
                            if state != ParserState::Top {
                                return Err(format!("Got <relation> element in state {:?}", state).into())
                            }

                            state = ParserState::Relation;
                            current_relation = ParseOsmEtl::parse_relation(e)?;
                        },
                        b"nd" => {
                            if state != ParserState::Way {
                                return Err(format!("Got <nd> element in state {:?}", state).into())
                            }

                            let id = ParseOsmEtl::parse_attr(e, b"ref")?;
                            current_way.nodes.push(
                                nodes.get(&id)
                                .ok_or::<Error>(format!("Reference to undefined node id {:?}", id).into())?
                                .clone()
                            );
                        },
                        b"member" => {
                            if state != ParserState::Relation {
                                return Err(format!("Got <member> element in state {:?}", state).into())
                            }

                            let member_type= ParseOsmEtl::get_attr(&e, b"type")?;
                            if b"way" != member_type.as_slice() {
                                continue
                            }

                            let id = ParseOsmEtl::parse_attr(e, b"ref")?;
                            current_relation.nodes.push(
                                nodes.get(&id)
                                .ok_or::<Error>(format!("Reference to undefined node id {:?}", id).into())?
                                .clone()
                            );
                        },
                        b"tag" => {
                            let key = ParseOsmEtl::get_attr(&e, b"key")?.to_vec();
                            let value = ParseOsmEtl::get_attr(&e, b"value")?.to_vec();

                            match state {
                                ParserState::Top => continue,
                                ParserState::Node => current_node.tags.insert(key, value),
                                ParserState::Way => current_way.tags.insert(key, value),
                                ParserState::Relation => current_relation.tags.insert(key, value),
                            };
                        },
                        _ => (),
                    }
                }
                Ok(Event::End(e)) => {
                    match e.name().as_ref() {
                        b"node" => {
                            state = ParserState::Top;
                            nodes.insert(current_node.id, current_node.clone());
                        },
                        b"way" => {
                            state = ParserState::Top;
                            ways.insert(current_way.id, current_way.clone());
                        },
                        b"relation" => {
                            state = ParserState::Top;
                            relations.insert(current_relation.id, current_relation.clone());
                        },
                        _ => {},
                    }
                },
                Ok(Event::Empty(e)) => {
                    if e.name().as_ref() == b"node" {
                        if let Ok(node) = ParseOsmEtl::parse_node(e) {
                            nodes.insert(node.id, node);
                        }
                    }
                },
                event => return Err(format!("Unexpected event {:?}", event).into()),
            }
            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        };
        Ok(Output {
            nodes,
            ways,
            relations,
        })
    }

    fn load(&self, mut output_file: fs::File, output: Self::Output) -> Result<()> {
        let bytes = rkyv::to_bytes::<_, 256>(&output.nodes).unwrap();
        output_file.write_all(&bytes)?;
        Ok(())
    }
}
