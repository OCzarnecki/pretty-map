use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::str;

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use xz::bufread::XzDecoder;

use crate::UserConfig;
use crate::data::osm::Node;
use crate::errors::Error;
use crate::etl::ETL;

const ETL_NAME: &str = "parse_osm";
const OUTPUT_FILE_NAME: &str = "osm_elements.rkyv";

pub struct Output {
    nodes: Vec<Node>,
}

enum ParserState {
    Top,
    Way,
    Relation,
}

pub struct ParseOSMETL<'a> {
    config: &'a UserConfig,
}

impl ParseOSMETL<'_> {
    fn parse_node(el: BytesStart) -> Option<Node> {
        let mut id: Option<u64> = None;
        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;

        for attribute_res in el.attributes() {
            let attribute = attribute_res.ok()?;
            match attribute.key.as_ref() {
                b"id" => {
                    let value_str = str::from_utf8(&attribute.value).ok()?;
                    id = Some(value_str.parse().ok()?);
                },
                b"lat" => {
                    let value_str = str::from_utf8(&attribute.value).ok()?;
                    lat = Some(value_str.parse().ok()?);
                },
                b"lon" => {
                    let value_str = str::from_utf8(&attribute.value).ok()?;
                    lon = Some(value_str.parse().ok()?);
                },
                b"version" => (),
                _ => {
                    eprintln!("WARNING: Unexpected attribute {:?}.", attribute.key);
                    return None
                },
            }
        }

        Some(Node {
            id: id?,
            lat: lat?,
            lon: lon?,
        })
    }

    fn create_osm_reader(&self) -> Result<Reader<impl BufRead>, Error> {
        let file = fs::File::open(Path::new("..").join(&self.config.data_path))?;
        let file_reader = BufReader::new(file);
        let xz_reader =  XzDecoder::new(file_reader);
        let buffered_xz_reader = BufReader::new(xz_reader);
        let mut reader = Reader::from_reader(buffered_xz_reader);
        reader.trim_text(true);

        Ok(reader)
    }

    pub fn new(config: &UserConfig) -> ParseOSMETL {
        ParseOSMETL {
            config
        }
    }
}

impl ETL for ParseOSMETL<'_> {
    type Input = ();
    type Output = Output;

    fn etl_name(&self) -> &str {
        ETL_NAME
    }

    fn output_file_name(&self) -> &str {
        OUTPUT_FILE_NAME
    }

    fn extract(&self) -> Result<Self::Input, Error> {
        Ok(())
    }

    fn transform(&self, _input: ()) -> Result<Self::Output, Error> {
        let mut reader = self.create_osm_reader()?;
        let mut buf = Vec::new();

        let mut nodes: Vec<Node> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => return Err(e.into()),
                Ok(Event::Eof) => break,
                Ok(Event::Decl(_e)) => (),
                Ok(Event::Text(_e)) => return Err("Didn't expect to see Text in OSM file.".into()),
                Ok(Event::Start(e)) => {
                    match e.name().as_ref() {
                        b"node" => {
                            if let Some(node) = ParseOSMETL::parse_node(e) {
                                nodes.push(node);
                            }
                        },
                        b"way" => (),
                        b"relation" => (),
                        _ => (),
                    }
                }
                Ok(Event::End(_e)) => (),
                Ok(Event::Empty(e)) => {
                    if e.name().as_ref() == b"node" {
                        if let Some(node) = ParseOSMETL::parse_node(e) {
                            nodes.push(node);
                        }
                    }
                },

                // There are several other `Event`s we do not consider here
                event => panic!("Unexpected event {:?}", event),
            }
            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        };
        Ok(Output {
            nodes,
        })
    }

    fn load(&self, mut output_file: fs::File, output: Self::Output) -> Result<(), Error> {
        let bytes = rkyv::to_bytes::<_, 256>(&output.nodes).unwrap();
        output_file.write_all(&bytes)?;
        Ok(())
    }
}
