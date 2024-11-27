use std::{collections::{HashMap, HashSet}, hash::Hash};
use super::osm::{Node, Way};

/// Collection of all semantic map elements (roads, tube stations, etc.) we could want to draw.

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Default, Clone)]
pub struct SemanticMapElements {
    pub underground_stations: Vec<TransportStation>,
    pub rails: Vec<Path>,
    pub roads: Vec<Path>,
    pub areas: Vec<Area>,
    pub landmarks: Vec<Landmark>,
    pub tube_rails: Vec<TubeRail>,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct MapCoords {
    pub lat: f64,
    pub lon: f64,
}

impl From<&Node> for MapCoords {
    fn from(value: &Node) -> Self {
        MapCoords {
            lat: value.lat,
            lon: value.lon,
        }
    }
}

impl Hash for MapCoords {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lat.to_bits().hash(state);
        self.lon.to_bits().hash(state);
    }
}

impl Eq for MapCoords { }

impl PartialEq for MapCoords {
    fn eq(&self, other: &Self) -> bool {
        self.lat == other.lat && self.lon == other.lon
    }
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct TransportStation {
    pub name: String,
    pub station_type: TransportStationType,
    pub lat: f64,
    pub lon: f64,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub enum TransportStationType {
    Underground,
    Overground,
    Dlr,
    ElizabethLine,
}

impl From<&TransportStation> for MapCoords {
    fn from(value: &TransportStation) -> Self {
        MapCoords {
            lat: value.lat,
            lon: value.lon,
        }
    }
}

pub type Path = Vec<MapCoords>;

impl From<&Way> for Path {
    fn from(value: &Way) -> Self {
        value.nodes.iter()
            .map(|el| el.into())
            .collect()
    }
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct Area {
    pub area_type: AreaType,
    pub area_polygons: Vec<Path>,
}

impl Area {
    fn reorder_ways(unordered_ways: &Vec<Path>) -> Vec<Path> {
        fn get_next(link_node: &MapCoords, way: &Path, by_end: &HashMap<MapCoords, Vec<Path>>) -> Option<(MapCoords, Path)> {
            let next_link = if *link_node == way[0] {
                &way[way.len() - 1]
            } else if *link_node == way[way.len() - 1] {
                &way[0]
            } else {
                panic!("Illegal state: link_node should be start or end of way");
            };

            let binding = Vec::new();
            let candidates = by_end.get(next_link).unwrap_or(&binding);

            if candidates.len() == 1 {
                None  // Self loop
            } else {
                for candidate in candidates {
                    if candidate != way {
                        return Some((next_link.clone(), candidate.clone()));
                    }
                }
                None  // Self-loop
            }
        }

        fn find_unseen(seen: &HashSet<Vec<MapCoords>>, ways: &Vec<Path>) -> Path {
            for way in ways {
                if !seen.contains(way) {
                    return way.clone();
                }
            }
            panic!("Illegal state: all ways have been seen");
        }

        if unordered_ways.is_empty() {
            panic!("Illegal state: 0-length way!");
        }

        let mut by_end: HashMap<MapCoords, Vec<Path>> = HashMap::new();
        for way in unordered_ways {
            by_end.entry(way[0].clone()).or_insert(Vec::new()).push(way.clone());
            by_end.entry(way[way.len() - 1].clone()).or_insert(Vec::new()).push(way.clone());
        }

        let mut ordered: Vec<Vec<Path>> = vec![vec![unordered_ways[0].clone()]];
        let mut link_node = ordered[0][0][0].clone();
        let mut seen: HashSet<Vec<MapCoords>> = HashSet::new();
        seen.insert(unordered_ways[0].clone());

        while ordered.iter().map(|group| group.len()).sum::<usize>() < unordered_ways.len() {
            if let Some((new_link_node, next_way)) = get_next(&link_node, &ordered[ordered.len() - 1][ordered[ordered.len() - 1].len() - 1], &by_end) {
                if !seen.contains(&next_way) {
                    ordered.last_mut().expect("Ordered was empty").push(next_way.clone());
                    seen.insert(next_way);
                    link_node = new_link_node;
                    continue;
                }
            }
            let next_way = find_unseen(&seen, unordered_ways);
            link_node = next_way[0].clone();
            ordered.push(vec![next_way.clone()]);
            seen.insert(next_way);
        }

        ordered.iter().map(|group| group.concat()).collect()
    }

    pub fn new(area_type: AreaType, unordered_ways: &Vec<Path>) -> Self {
        Area {
            area_type,
            area_polygons: Area::reorder_ways(unordered_ways),
        }
    }
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub enum AreaType {
    Park,
    Water,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct Landmark {
    pub lon: f64,
    pub lat: f64,
    pub landmark_type: LandmarkType,
}

impl From<&Landmark> for MapCoords {
    fn from(value: &Landmark) -> Self {
        MapCoords {
            lat: value.lat,
            lon: value.lon,
        }
    }
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub enum LandmarkType {
    Lgbtq,
    LgbtqMen,
    CocktailBar,
    ClimbingBoulder,
    ClimbingRope,
    ClimbingOutdoor,
    Gym,
    Hospital,
    MusicVenue,
    Tree,
    TubeEmergencyExit,
    TempleAetheriusSociety,
    TempleBuddhist,
    TempleChristian,
    TempleHindu,
    TempleHumanist,
    TempleJain,
    TempleJewish,
    TempleMuslim,
    TempleRastafarian,
    TempleRosicucian,
    TempleScientologist,
    TempleSelfRealizationFellowship,
    TempleSikh,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
pub struct TubeRail {
    pub line: TubeLine,
    pub path: Path,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum TubeLine {
    Bakerloo,
    Central,
    Circle,
    District,
    Dlr,
    Elizabeth,
    HammersmithAndCity,
    Jubilee,
    Metropolitan,
    Northern,
    Overground,  // Not technically a tube line, sue me
    Piccadilly,
    Victoria,
    WaterlooAndCity,
}
