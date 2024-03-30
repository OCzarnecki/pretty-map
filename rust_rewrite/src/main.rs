mod etl_engine;

use std::fs::{create_dir_all, File};
use std::io::{BufRead, BufReader, Write, Read};
use std::path::Path;
use std::str;

use serde::{Deserialize, Serialize};

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use rkyv;
use xz::bufread::XzDecoder;

#[derive(Deserialize)]
struct UserConfig {
    data_path: String,
    dest_path: String,
    top_left_lon: f64,
    top_left_lat: f64,
    px_per_deg_lon: f64,
    px_per_deg_lat: f64,
    width_px: u64,
    height_px: u64,
}

struct PathConfig {
    data_path: Box<Path>,
    output_path: Box<Path>,
    node_cache_path: Box<Path>,
    trimmed_map_spec: Box<Path>,
    success_file_path: Box<Path>,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug)]
struct Node {
    id: u64,
    lon: f64,
    lat: f64,
}

fn create_paths(config: &UserConfig) -> PathConfig {
    let input_fname = Path::new(&config.data_path).file_name().expect("Could not get data file name");
    let cache_dir_name = "cache_".to_string() + input_fname.to_str().unwrap();
    let cache_dir_path = Path::new(&cache_dir_name);
    let path_config = PathConfig {
        data_path: Path::new(&config.data_path).into(),
        output_path: Path::new(&config.dest_path).into(),
        node_cache_path: cache_dir_path.join("nodes").into(),
        trimmed_map_spec: cache_dir_path.join("trimmed_map_data").into(),
        success_file_path: cache_dir_path.join("cache_creation_finished").into(),
    };

    create_dir_all(cache_dir_path).expect("Could not create cache dir.");

    path_config
}

fn is_cache_populated(path_config: &PathConfig) -> bool {
    path_config.success_file_path.exists()
}

fn update_status(count: u64) {
    if count % 100000 == 0 {
        print!("XML elements read: {:10}.\r", count);
        std::io::stdout().flush().expect("Couldn't flush stdout.");
    }
}

fn create_reader(path: &Path) -> Reader<impl BufRead> {
    let file = File::open(Path::new("..").join(path)).expect("Could not read OSM file.");
    let file_reader = BufReader::new(file);
    let xz_reader =  XzDecoder::new(file_reader);
    let buffered_xz_reader = BufReader::new(xz_reader);
    let mut reader = Reader::from_reader(buffered_xz_reader);
    reader.trim_text(true);

    reader
}

fn load_user_config(path: &str) -> UserConfig {
    let file = File::open(path).expect("Could not open config file.");
    serde_json::from_reader(file).expect("Could not parse config.")
}

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

fn populate_cache(path_config: &PathConfig) {
    let mut reader = create_reader(&path_config.data_path);
    let mut buf = Vec::new();
    let mut count = 0;

    let mut nodes: Vec<Node> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            // exits the loop when reaching end of file
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"osm" => (),
                    b"node" => {
                        if let Some(node) = parse_node(e) {
                            nodes.push(node);
                        }
                        
                        count += 1;
                        update_status(count);
                    },
                    name => {
                        // println!("{:?}", str::from_utf8(name));
                        break
                    },
                }
            }
            Ok(Event::End(_e)) => (),
            Ok(Event::Text(_e)) => panic!("Didn't expect to see Text in OSM file."),
            Ok(Event::Decl(_e)) => (),
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"node" {
                    if let Some(node) = parse_node(e) {
                        nodes.push(node);
                    }
                }
                count += 1;
                update_status(count);
                // println!("{:?}", str::from_utf8(e.name().as_ref()));
            },

            // There are several other `Event`s we do not consider here
            event => panic!("Unexpected event {:?}", event),
        }
        reader.into_inner()
        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    eprintln!();

    let mut fout = File::create(&path_config.node_cache_path).expect("Could not open node cache file.");
    let bytes = rkyv::to_bytes::<_, 256>(&nodes).unwrap();
    fout.write_all(&bytes).expect("Could not write node cache.");
    // write(&mut fout, &nodes).expect("Error while serializing node vector");

    File::create(&path_config.success_file_path).expect("Could not create success file.");
}

fn main() {
    let user_config = load_user_config("../config/london_full.json");
    let path_config = create_paths(&user_config);
    if !is_cache_populated(&path_config) {
        eprintln!("Generating cache...");
        populate_cache(&path_config);
        eprintln!("Finished creating cache.");
    }

    let mut fin = File::open(&path_config.node_cache_path).expect("Could not open node cache file.");
    let mut buf_vec: Vec<u8> = Vec::new();
    fin.read_to_end(&mut buf_vec).expect("Could not read note cache.");
    let nodes: Vec<Node> = unsafe {
        rkyv::from_bytes_unchecked(&buf_vec).expect("Could not deserialize node cache.")
    };
    eprintln!("Read {} nodes from cache.", nodes.len());


}
