use std::{str, collections::HashMap, fs::{self, File}, io::{Read, Write}, path::{Path, PathBuf}};

use crate::{
    data::{
        osm::{Node, OsmId, OsmMapData, Relation, Way},
        semantic::{
            Area,
            AreaType,
            Landmark,
            LandmarkType,
            SemanticMapElements,
            TransportStation,
            TransportStationType,
            TubeLine,
            TubeRail,
        }
    },
    errors::Result,
};
use crate::etl::parse_osm;

use super::Etl;
use quick_xml::escape::unescape;
use regex::Regex;

pub const ETL_NAME: &str = "semantic_map";
pub const OUTPUT_FILE_NAME: &str = "semantic_map.rkyv";

pub struct SemanticMapEtl {
}

impl SemanticMapEtl {
    fn output_path(dir: &Path) -> PathBuf {
        dir.join(OUTPUT_FILE_NAME)
    }

    fn has_key(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8]) -> bool {
        let key_vec = key.to_vec();
        tags.contains_key(&key_vec)
    }

    fn has_kv_pair(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8], value: &[u8]) -> bool {
        let key_vec = key.to_vec();
        if let Some(tag_value) = tags.get(&key_vec) {
            tag_value.split(|b| *b == 59)
                .any(|tag| tag == value)
        } else {
            false
        }
    }

    fn get_string(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8]) -> Option<String> {
        let key_vec = key.to_vec();
        let val_vec = tags.get(&key_vec)?;
        Some(
            unescape(
                str::from_utf8(val_vec).ok()?
            ).ok()?.to_string()
        )
    }

    fn get_strings(tags: &HashMap<Vec<u8>, Vec<u8>>, key: &[u8]) -> Vec<String> {
        Self::get_string(tags, key)
            .map(|s| s.split(";").map(|s| s.to_string()).collect::<Vec<String>>())
            .into_iter()
            .flatten()
            .collect()
    }

    fn landmark_type_from_tags(&mut self, tags: &HashMap<Vec<u8>, Vec<u8>>) -> Option<LandmarkType> {
        if Self::has_kv_pair(tags, b"lgbtq:men", b"only")
            || Self::has_kv_pair(tags, b"lgbtq:men", b"primary")
            || Self::has_kv_pair(tags, b"gay", b"yes") {
                Some(LandmarkType::LgbtqMen)
        } else if Self::has_kv_pair(tags, b"lgbtq", b"primary") {
            Some(LandmarkType::Lgbtq)
        } else if Self::has_kv_pair(tags, b"bar", b"cocktail")
            || Self::has_kv_pair(tags, b"cocktails", b"yes")
            || Self::has_kv_pair(tags, b"drink:cocktail", b"served") {
            Some(LandmarkType::CocktailBar)
        } else if Self::has_kv_pair(tags, b"emergency", b"emergency_ward_entrance")
            || Self::has_kv_pair(tags, b"healthcare", b"emergency_ward") {
            Some(LandmarkType::Hospital)
        } else if Self::has_kv_pair(tags, b"natural", b"tree")
            && Self::has_key(tags, b"name") {
            Some(LandmarkType::Tree)
        } else if Self::has_kv_pair(tags, b"leisure", b"fitness_centre") {
            Some(LandmarkType::Gym)
        } else if Self::has_kv_pair(tags, b"climbing:toprope", b"yes")
            || Self::has_kv_pair(tags, b"climbing:sport", b"yes")
            || Self::has_kv_pair(tags, b"climbing:ice", b"yes") {
            Some(LandmarkType::ClimbingRope)
        } else if Self::has_kv_pair(tags, b"climbing:boulder", b"yes")
            || Self::has_kv_pair(tags, b"climbing", b"bouldering") 
            || (
                Self::has_kv_pair(tags, b"leisure", b"sports_centre")
                && Self::has_kv_pair(tags, b"sport", b"climbing")
            ) {
            Some(LandmarkType::ClimbingBoulder)
        } else if Self::has_kv_pair(tags, b"leisure", b"pitch")
            && Self::has_kv_pair(tags, b"sport", b"climbing") {
            Some(LandmarkType::ClimbingOutdoor)
        } else if Self::has_kv_pair(tags, b"amenity", b"music_venue")
            || Self::has_kv_pair(tags, b"live_music", b"yes") {
            Some(LandmarkType::MusicVenue)
        } else if Self::has_kv_pair(tags, b"amenity", b"place_of_worship") {
            if Self::has_kv_pair(tags, b"religion", b"aetherius_society") {
                Some(LandmarkType::TempleAetheriusSociety)
            } else if Self::has_kv_pair(tags, b"religion", b"buddhist") {
                Some(LandmarkType::TempleBuddhist)
            } else if Self::has_kv_pair(tags, b"religion", b"christian") || Self::has_kv_pair(tags, b"religion", b"spiritualist") {
                Some(LandmarkType::TempleChristian)
            } else if Self::has_kv_pair(tags, b"religion", b"hindu") {
                Some(LandmarkType::TempleHindu)
            } else if Self::has_kv_pair(tags, b"religion", b"humanist") {
                Some(LandmarkType::TempleHumanist)
            } else if Self::has_kv_pair(tags, b"religion", b"jain") {
                Some(LandmarkType::TempleJain)
            } else if Self::has_kv_pair(tags, b"religion", b"jewish") {
                Some(LandmarkType::TempleJewish)
            } else if Self::has_kv_pair(tags, b"religion", b"muslim") {
                Some(LandmarkType::TempleMuslim)
            } else if Self::has_kv_pair(tags, b"religion", b"rastafarian") {
                Some(LandmarkType::TempleRastafarian)
            } else if Self::has_kv_pair(tags, b"religion", b"rosicrucian") {
                Some(LandmarkType::TempleRosicucian)
            } else if Self::has_kv_pair(tags, b"religion", b"scientologist") {
                Some(LandmarkType::TempleScientologist)
            } else if Self::has_kv_pair(tags, b"religion", b"self-realization_fellowship") {
                Some(LandmarkType::TempleSelfRealizationFellowship)
            } else if Self::has_kv_pair(tags, b"religion", b"sikh") {
                Some(LandmarkType::TempleSikh)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn process_nodes(&mut self, output: &mut SemanticMapElements, nodes: &HashMap<OsmId, Node>) {
        for node in nodes.values() {
            if Self::has_kv_pair(&node.tags, b"railway", b"station")
                // && Self::has_kv_pair(&node.tags, b"subway", b"yes")
                && Self::has_key(&node.tags, b"name") {

                let name = Self::get_string(&node.tags, b"name").unwrap();

                // Some names are like "Edgeware Road (Bakerloo line)", we want to strip the
                // brackets.
                let re = Regex::new(r"(?<base_name>[^(]*)(\(.*\))?").unwrap();
                let base_name = re.captures(&name).unwrap().name("base_name").unwrap().as_str();
                let maybe_station_type = if Self::has_kv_pair(&node.tags, b"network", b"London Underground") {
                    Some(TransportStationType::Underground)
                } else if Self::has_kv_pair(&node.tags, b"network", b"Docklands Light Railway"){
                    Some(TransportStationType::Dlr)
                } else if Self::has_kv_pair(&node.tags, b"network", b"London Overground") {
                    Some(TransportStationType::Overground)
                } else if Self::has_kv_pair(&node.tags, b"network", b"Elizabeth Line") {
                    Some(TransportStationType::ElizabethLine)
                } else {
                    None
                };
                if let Some(station_type) = maybe_station_type {
                    output.underground_stations.push(
                        TransportStation {
                            name: base_name.trim().to_string(),
                            station_type,
                            lon: node.lon,
                            lat: node.lat
                        }
                    );
                }
            }
            if let Some(landmark_type) = self.landmark_type_from_tags(&node.tags) {
                output.landmarks.push(
                    Landmark{
                        lon: node.lon,
                        lat: node.lat,
                        landmark_type,
                    }
                );
            }

            // Bobby Fitzpatric
            if node.id == 5417354028 {
                output.landmarks.push(
                    Landmark{
                        lon: node.lon,
                        lat: node.lat,
                        landmark_type: LandmarkType::CocktailBar,
                    }
                );
            }
        }
    }

    fn process_ways(&mut self, output: &mut SemanticMapElements, ways: &HashMap<OsmId, Way>) {
        for way in ways.values() {
            if Self::has_key(&way.tags, b"highway") {
                output.roads.push(way.into());
            }
            if Self::has_kv_pair(&way.tags, b"leisure", b"park") {
                output.areas.push(
                    Area::new(
                        AreaType::Park,
                        &vec![way.into()],
                    )
                );
            }
            if Self::has_key(&way.tags, b"water") 
                || Self::has_kv_pair(&way.tags, b"natural", b"water")
            {
                output.areas.push(
                    Area::new(
                        AreaType::Water,
                        &vec![way.into()],
                    )
                );
            }
            if (
                Self::has_kv_pair(&way.tags, b"railway", b"subway")
                || Self::has_kv_pair(&way.tags, b"railway", b"rail")
                || Self::has_kv_pair(&way.tags, b"railway", b"light_rail")
            ) && Self::has_key(&way.tags, b"line") {
                for line_tag in Self::get_strings(&way.tags, b"line") {
                    let lines = match line_tag.to_lowercase().as_str() {
                        "bakerloo" => vec![TubeLine::Bakerloo],
                        "central" | "central line" => vec![TubeLine::Central],
                        "circle" => vec![TubeLine::Circle],
                        "deep level district" | "district" | "district, north london" => vec![TubeLine::District],
                        "district, piccadilly" => vec![TubeLine::District, TubeLine::Piccadilly],
                        "dlr" => vec![TubeLine::Dlr],
                        "elizabeth" => vec![TubeLine::Elizabeth],
                        "hammersmith & city" => vec![TubeLine::HammersmithAndCity],
                        "jubilee" | "jubilee line" => vec![TubeLine::Jubilee],
                        "metropolitan" => vec![TubeLine::Metropolitan],
                        "metropolitan, piccadilly" => vec![TubeLine::Metropolitan, TubeLine::Piccadilly],
                        "north london line" => vec![TubeLine::Overground],
                        "northern" | "northern line" => vec![TubeLine::Northern],
                        "northern city" => vec![TubeLine::Northern, TubeLine::HammersmithAndCity],
                        "picadilly" | "piccadilly" => vec![TubeLine::Piccadilly],
                        "victoria" => vec![TubeLine::Victoria],
                        "waterloo & city" => vec![TubeLine::WaterlooAndCity],
                        _ => {
                            vec![]
                        },
                    };
                    for line in lines {
                        output.tube_rails.push(TubeRail {
                            line,
                            path: way.into(),
                        });
                    }
                }
            } else if Self::has_kv_pair(&way.tags, b"railway", b"rail")
                && Self::has_kv_pair(&way.tags, b"name", b"Elizabeth Line") {
                output.tube_rails.push(TubeRail {
                    line: TubeLine::Elizabeth,
                    path: way.into(),
                });
            // } else if Self::has_kv_pair(&way.tags, b"railway", b"rail")
            //     && (
            //         Self::has_kv_pair(&way.tags, b"network", b"London Overground")
            //         || Self::has_kv_pair(&way.tags, b"line", b"Lea Valley Lines")
            //         || Self::has_kv_pair(&way.tags, b"line", b"North London Line")
            //         || Self::has_kv_pair(&way.tags, b"line", b"North London line")
            //         || Self::has_kv_pair(&way.tags, b"name", b"East London Line")
            //         || Self::has_kv_pair(&way.tags, b"name", b"East London line")
            //         || Self::has_kv_pair(&way.tags, b"name", b"Hackney Downs and Cheshunt Line")
            //         || Self::has_kv_pair(&way.tags, b"name", b"North/West London lines")
            //         || Self::has_kv_pair(&way.tags, b"name", b"South Bermondsey to Horsham Line")
            //         || Self::has_kv_pair(&way.tags, b"name", b"South London Line")
            //         || Self::has_kv_pair(&way.tags, b"name", b"South London line")
            //         || Self::has_kv_pair(&way.tags, b"name", b"South/West London lines")
            //         || Self::has_kv_pair(&way.tags, b"name", b"West Anglia Main Line")
            //     ) {
            //     output.tube_rails.push(TubeRail {
            //         line: TubeLine::Overground,
            //         path: way.into(),
            //     });
            } else if Self::has_kv_pair(&way.tags, b"railway", b"rail") {
                output.rails.push(way.into());
            }

            if let Some(landmark_type) = self.landmark_type_from_tags(&way.tags) {
                output.landmarks.push(
                    Landmark{
                        lon: way.nodes[0].lon,
                        lat: way.nodes[0].lat,
                        landmark_type,
                    }
                );
            }

            // Handle castle climbing separately
            if way.id == 963992061 {
                output.landmarks.push(
                    Landmark{
                        lon: way.nodes[0].lon,
                        lat: way.nodes[0].lat,
                        landmark_type: LandmarkType::ClimbingRope,
                    }
                );
            }
        }
    }

    fn process_relations(&mut self, output: &mut SemanticMapElements, relations: &HashMap<OsmId, Relation>) {
        for relation in relations.values() {
            if Self::has_kv_pair(&relation.tags, b"leisure", b"park") {
                output.areas.push(
                    Area::new(
                        AreaType::Park,
                        &relation.ways.iter().map(|way| way.into()).collect(),
                    )
                );
            }
            if Self::has_key(&relation.tags, b"water")
                || Self::has_kv_pair(&relation.tags, b"natural", b"water")
            {
                output.areas.push(
                    Area::new(
                        AreaType::Water,
                        &relation.ways.iter().map(|way| way.into()).collect(),
                    )
                );
            }
            if Self::has_kv_pair(&relation.tags, b"network", b"London Overground") {
                for way in &relation.ways {
                    if Self::has_kv_pair(&way.tags, b"railway", b"rail") {
                        output.tube_rails.push(TubeRail {
                            line: TubeLine::Overground,
                            path: way.into(),
                        });
                    }
                }
            }
        }
    }

    pub fn new() -> SemanticMapEtl {
        SemanticMapEtl {}
    }
}

impl Etl for SemanticMapEtl {
    type Input = OsmMapData;
    type Output = SemanticMapElements;

    fn etl_name(&self) -> &str {
        ETL_NAME
    }

    fn is_cached(&self, dir: &std::path::Path) -> Result<bool> {
        Ok(Self::output_path(dir).exists())
    }

    fn clean(&self, dir: &std::path::Path) -> Result<()> {
        fs::remove_file(Self::output_path(dir))?;
        Ok(())
    }

    fn extract(&mut self, dir: &std::path::Path) -> Result<Self::Input> {
        let input_file_path = dir.join(parse_osm::OUTPUT_FILE_NAME);
        let mut input_file = File::open(input_file_path)?;

        let mut buf_vec: Vec<u8> = Vec::new();
        input_file.read_to_end(&mut buf_vec).expect("Could not read note cache.");

        let input: OsmMapData = unsafe {
            rkyv::from_bytes_unchecked(&buf_vec).expect("Could not deserialize node cache.")
        };

        Ok(input)
    }

    fn transform(&mut self, input: Self::Input) -> Result<Self::Output> {
        let mut output = SemanticMapElements::default();

        self.process_nodes(&mut output, &input.nodes);
        self.process_ways(&mut output, &input.ways);
        self.process_relations(&mut output, &input.relations);

        Ok(output)
    }

    fn load(&mut self, dir: &std::path::Path, output: Self::Output) -> Result<()> {
        let mut output_file = File::create(Self::output_path(dir))?;
        let bytes = rkyv::to_bytes::<_, 256>(&output).unwrap();
        output_file.write_all(&bytes)?;
        Ok(())
    }

}
