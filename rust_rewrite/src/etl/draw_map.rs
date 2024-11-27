use core::f64;
use std::{cmp::Ordering, fs::{self, File}, io::Read, path::{Path, PathBuf}};

use png::{self, BitDepth, ColorType};
use raqote::{BlendMode, DrawOptions, DrawTarget, Image, LineCap, LineJoin, PathBuilder, Point, SolidSource, Source, StrokeStyle};
use serde::Deserialize;

use crate::{
    data::semantic::{
        self, Area, Landmark, MapCoords, SemanticMapElements, TransportStation
    },
    errors::Result, UserConfig,
};

use super::{semantic_map, Etl};

mod fk {
    pub use font_kit::font::Font;
    pub use pathfinder_geometry::vector::vec2f;
}

pub const ETL_NAME: &str = "draw_map";
pub const OUTPUT_FILE_NAME: &str = "output.png";

enum PathStyle {
    Road,
    Rail,
}

pub struct OwnedImage {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u32>,
}

use serialize_color::deserialize;

#[derive(Deserialize)]
pub struct Theme<'a> {
    #[serde(deserialize_with = "deserialize")]
    pub background_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub park_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub road_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub rail_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub text_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub water_color: Source<'a>,
}

mod serialize_color {
    use raqote::{SolidSource, Source};
    use serde::{de, Deserializer};
    use serde::de::Visitor;


    struct ColorVisitor;

    impl<'de> Visitor<'de> for ColorVisitor {
        type Value = SolidSource;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a JSON dictionary containg 'r', 'g', 'b', and 'a' keys")
        }

        fn visit_str<E>(self, string: &str) -> Result<Self::Value, E> where E: de::Error {
            if string.len() != 9 {
                return Err(de::Error::invalid_value(de::Unexpected::Str(string), &self))
            }
            let r = parse_hex_byte(&self, &string[1..3])?;
            let g = parse_hex_byte(&self, &string[3..5])?;
            let b = parse_hex_byte(&self, &string[5..7])?;
            let a = parse_hex_byte(&self, &string[7..9])?;
            Ok(SolidSource::from_unpremultiplied_argb(a, r, g, b))
        }
    }

    fn parse_hex_byte<E>(visitor: &ColorVisitor, string: &str) -> Result<u8, E> where E: de::Error {
        u8::from_str_radix(string, 16).map_err(|_| {
            de::Error::invalid_value(de::Unexpected::Str(string), visitor)
        })
    }

    pub fn deserialize<'de, 'a, D>(
        deserializer: D,
    ) -> Result<Source<'a>, D::Error>
        where D: Deserializer<'de>, 'a: 'de {
        Ok(Source::Solid(deserializer.deserialize_str(ColorVisitor)?))
    }
}

pub struct DrawMapEtl <'a> {
    user_config: &'a UserConfig<'a>,
    underground_logo: OwnedImage,
    overground_logo: OwnedImage,
    dlr_logo: OwnedImage,
    elizabeth_line_logo: OwnedImage,
    lgbtq_logo: OwnedImage,
    cocktail_logo: OwnedImage,
    climbing_boulder_logo: OwnedImage,
    climbing_rope_logo: OwnedImage,
    climbing_outdoor_logo: OwnedImage,
    gym_logo: OwnedImage,
    hospital_logo: OwnedImage,
    music_logo: OwnedImage,
    temple_aetherius_society_logo: OwnedImage,
    temple_buddhist_logo: OwnedImage,
    temple_christian_logo: OwnedImage,
    temple_hindu_logo: OwnedImage,
    temple_humanist_logo: OwnedImage,
    temple_jain_logo: OwnedImage,
    temple_jewish_logo: OwnedImage,
    temple_muslim_logo: OwnedImage,
    temple_rastafarian_logo: OwnedImage,
    temple_rosicrucian_logo: OwnedImage,
    temple_scientologist_logo: OwnedImage,
    temple_self_realization_fellowship_logo: OwnedImage,
    temple_sikh_logo: OwnedImage,
    tree_logo: OwnedImage,
    x_shift: f32,
    y_shift: f32,
    font: fk::Font,
    theme: &'a Theme<'a>,
}

impl DrawMapEtl<'_> {
    fn output_path(dir: &Path) -> PathBuf {
        dir.join(OUTPUT_FILE_NAME)
    }

    fn project_mercantor(&self, coords: &MapCoords) -> (f32, f32) {
        let rel_lon = coords.lon - self.user_config.top_left_lon;
        let rel_lat = coords.lat - self.user_config.top_left_lat;

        let x = rel_lon * self.user_config.px_per_deg_lon;
        let y = - rel_lat * self.user_config.px_per_deg_lat;
        (x as f32 - self.x_shift, y as f32 - self.y_shift)
    }

    fn stroke(width: f32) -> StrokeStyle {
        StrokeStyle {
            cap: LineCap::Round,
            join: LineJoin::Round,
            width,
            miter_limit: 2.0,
            dash_array: Vec::new(),
            dash_offset: 0.0,
        }
    }

    fn draw_semantic_path(&self, dt: &mut DrawTarget, semantic_path: &semantic::Path, style: &PathStyle) {
        if semantic_path.len() < 2 {
            return;
        }
        let mut pb = PathBuilder::new();
        let (x0, y0) = self.project_mercantor(&semantic_path[0]);
        pb.move_to(x0, y0);

        for coords in &semantic_path[1..] {
            let (x, y) = self.project_mercantor(coords);
            pb.line_to(x, y);
        }
        let raquote_path = pb.finish();

        let draw_options = DrawOptions::new();

        match style {
            PathStyle::Road => {
                dt.stroke(
                    &raquote_path,
                    &self.theme.road_color,
                    &Self::stroke(6.0),
                    &draw_options,
                );
            },
            PathStyle::Rail => {
                dt.stroke(
                    &raquote_path,
                    &self.theme.rail_color,
                    &StrokeStyle {
                        cap: LineCap::Round,
                        join: LineJoin::Round,
                        width: 3.0,
                        miter_limit: 2.0,
                        dash_array: vec![7.5, 13.5],
                        dash_offset: 12.0,
                    },
                    &draw_options,
                );
            },
        }
    }

    fn draw_tube_rail(&self, dt: &mut DrawTarget, tube_rail: &semantic::TubeRail) {
        fn wiggle(n: u64, n_max: u64, r: f64) -> (f64, f64) {
            let angle = (n as f64) / (n_max as f64) * f64::consts::TAU;
            (angle.cos() * r, angle.sin() * r)
        }

        let semantic_path = &tube_rail.path;
        if semantic_path.len() < 2 {
            return;
        }
        let mut pb = PathBuilder::new();

        let offset_radius = 30.0;
        let (dx, dy) = match tube_rail.line {
            semantic::TubeLine::Bakerloo => wiggle(0, 14, offset_radius),
            semantic::TubeLine::Central => wiggle(1, 14, offset_radius),
            semantic::TubeLine::Circle => wiggle(2, 14, offset_radius),
            semantic::TubeLine::District => wiggle(3, 14, offset_radius),
            semantic::TubeLine::Dlr => wiggle(4, 14, offset_radius),
            semantic::TubeLine::Elizabeth => wiggle(5, 14, offset_radius),
            semantic::TubeLine::HammersmithAndCity => wiggle(6, 14, offset_radius),
            semantic::TubeLine::Jubilee => wiggle(7, 14, offset_radius),
            semantic::TubeLine::Metropolitan => wiggle(8, 14, offset_radius),
            semantic::TubeLine::Northern => wiggle(9, 14, offset_radius),
            semantic::TubeLine::Overground => wiggle(10, 14, offset_radius),
            semantic::TubeLine::Piccadilly => wiggle(11, 14, offset_radius),
            semantic::TubeLine::Victoria => wiggle(12, 14, offset_radius),
            semantic::TubeLine::WaterlooAndCity => wiggle(13, 14, offset_radius),
        };

        let (dx, dy): (f32, f32) = (dx as f32, dy as f32);

        let (x0, y0) = self.project_mercantor(
            &semantic_path[0]
            // &MapCoords {
                // lat: semantic_path[0].lat + dy,
                // lon: semantic_path[1].lon + dx,
            // }
        );
        pb.move_to(x0 + dx, y0 + dy);

        for coords in &semantic_path[1..] {
            let (x, y) = self.project_mercantor(
                coords,
                // &MapCoords {
                    // lat: coords.lat + dy,
                    // lon: coords.lon + dx,
                // }
            );
            pb.line_to(x + dx, y + dy);
        }
        let raquote_path = pb.finish();

        let draw_options = DrawOptions::new();

        dt.stroke(
            &raquote_path,
            &Source::Solid(
                match tube_rail.line {
                    semantic::TubeLine::Bakerloo => SolidSource::from_unpremultiplied_argb(0xff, 0x89, 0x4e, 0x24),
                    semantic::TubeLine::Central => SolidSource::from_unpremultiplied_argb(0xff, 0xDC, 0x24, 0x1f),
                    semantic::TubeLine::Circle => SolidSource::from_unpremultiplied_argb(0xff, 0xFF, 0xCE, 0x00),
                    semantic::TubeLine::District => SolidSource::from_unpremultiplied_argb(0xff, 0x00, 0x72, 0x29),
                    semantic::TubeLine::Dlr => SolidSource::from_unpremultiplied_argb(0xff, 0x00, 0xaf, 0xad),
                    semantic::TubeLine::Elizabeth => SolidSource::from_unpremultiplied_argb(0xff, 0x69, 0x50, 0xa1),
                    semantic::TubeLine::HammersmithAndCity => SolidSource::from_unpremultiplied_argb(0xff, 0xd7, 0x99, 0xaf),
                    semantic::TubeLine::Jubilee => SolidSource::from_unpremultiplied_argb(0xff, 0x6a, 0x72, 0x78),
                    semantic::TubeLine::Metropolitan => SolidSource::from_unpremultiplied_argb(0xff, 0x75, 0x10, 0x56),
                    semantic::TubeLine::Northern => SolidSource::from_unpremultiplied_argb(0xff, 0x00, 0x00, 0x00),
                    semantic::TubeLine::Overground => SolidSource::from_unpremultiplied_argb(0xff, 0xe8, 0x6a, 0x10),
                    semantic::TubeLine::Piccadilly => SolidSource::from_unpremultiplied_argb(0xff, 0x00, 0x19, 0xa8),
                    semantic::TubeLine::Victoria => SolidSource::from_unpremultiplied_argb(0xff, 0x00, 0xa0, 0xe2),
                    semantic::TubeLine::WaterlooAndCity => SolidSource::from_unpremultiplied_argb(0xff, 0x76, 0xd0, 0xbd),
                }
            ),
            &Self::stroke(5.0),
            &draw_options,
        );
    }

    fn draw_text(
        &self,
        dt: &mut DrawTarget,
        x: f32,
        y: f32,
        point_size: f32,
        text: &str,
    ) {
        let source = &self.theme.text_color;
        let options = DrawOptions::new();
        let mut start = fk::vec2f(x, y);
        let mut ids = Vec::new();
        let mut positions = Vec::new();
        for c in text.chars() {
            let id = self.font.glyph_for_char(c).unwrap();
            ids.push(id);
            positions.push(Point::new(start.x(), start.y()));
            start += self.font.advance(id).unwrap() * point_size / 24. / 96. * 2.0;
        }
        let total_width: f32 = positions[positions.len() - 1].x - x + point_size / 2.0;
        for position in &mut positions {
            position.x -= total_width * 0.5;
        }
        dt.draw_glyphs(&self.font, point_size, &ids, &positions, source, &options);
    }

    fn draw_undergound_station(&self, dt: &mut DrawTarget, station: &TransportStation) {
        let (x_center, y_center) = self.project_mercantor(&station.into());
        let width = 94.0;
        let height = 78.0;

        let logo = match station.station_type {
            semantic::TransportStationType::Underground => &self.underground_logo,
            semantic::TransportStationType::Overground => &self.overground_logo,
            semantic::TransportStationType::Dlr => &self.dlr_logo,
            semantic::TransportStationType::ElizabethLine => &self.elizabeth_line_logo,
        };

        let img = Image {
            width: self.underground_logo.width,
            height: self.underground_logo.height,
            data: &logo.data,
        };

        let mut draw_options = DrawOptions::new();
        draw_options.blend_mode = BlendMode::SrcOver;

        dt.draw_image_with_size_at(
            width,
            height,
            x_center - width / 2.0,
            y_center - height / 2.0,
            &img,
            &draw_options,
        );

        self.draw_text(dt, x_center, y_center + height / 2.0 + 15.0, 20.0, &station.name);
        // dt.draw_image_at(x_center, y_center, &img, &DrawOptions::new());
    }

    pub fn new<'a>(user_config: &'a UserConfig<'a>) -> DrawMapEtl<'a> {
        let font = font_kit::loader::Loader::from_file(
            &mut std::fs::File::open("resources/fonts/Domine-Bold.ttf").unwrap(), 0
        ).unwrap();

        DrawMapEtl {
            user_config,
            underground_logo: Self::load_image("ug_2").unwrap(),
            overground_logo: Self::load_image("overground").unwrap(),
            elizabeth_line_logo: Self::load_image("elizabeth").unwrap(),
            dlr_logo: Self::load_image("dlr").unwrap(),
            gym_logo: Self::load_image("gym").unwrap(),
            lgbtq_logo: Self::load_image("lgbtq").unwrap(),
            cocktail_logo: Self::load_image("cocktail").unwrap(),
            climbing_boulder_logo: Self::load_image("climbing_boulder2").unwrap(),
            climbing_rope_logo: Self::load_image("climbing_rope2").unwrap(),
            climbing_outdoor_logo: Self::load_image("climbing_outdoor").unwrap(),
            hospital_logo: Self::load_image("hospital").unwrap(),
            music_logo: Self::load_image("music_venue").unwrap(),
            font,
            theme: &user_config.theme,
            temple_aetherius_society_logo: Self::load_image("aetherius_society").unwrap(),
            temple_buddhist_logo: Self::load_image("buddhist-stupa").unwrap(),
            temple_christian_logo: Self::load_image("crucifix").unwrap(),
            temple_hindu_logo: Self::load_image("hindu-om").unwrap(),
            temple_humanist_logo: Self::load_image("humanism").unwrap(),
            temple_jain_logo: Self::load_image("janism").unwrap(),
            temple_jewish_logo: Self::load_image("judaism").unwrap(),
            temple_muslim_logo: Self::load_image("islam").unwrap(),
            temple_rastafarian_logo: Self::load_image("rastafarianism").unwrap(),
            temple_rosicrucian_logo: Self::load_image("rosicrucianism").unwrap(),
            temple_scientologist_logo: Self::load_image("scientology").unwrap(),
            temple_self_realization_fellowship_logo: Self::load_image("self-realization-fellowship").unwrap(),
            temple_sikh_logo: Self::load_image("sikh").unwrap(),
            tree_logo: Self::load_image("tree").unwrap(),
            x_shift: 0.0,
            y_shift: 0.0,
        }
        //DrawMapEtl {
        //    user_config,
        //    underground_logo: Self::load_image("lgbtq").unwrap(),
        //    overground_logo: Self::load_image("lgbtq").unwrap(),
        //    elizabeth_line_logo: Self::load_image("lgbtq").unwrap(),
        //    dlr_logo: Self::load_image("lgbtq").unwrap(),
        //    lgbtq_logo: Self::load_image("lgbtq").unwrap(),
        //    cocktail_logo: Self::load_image("lgbtq").unwrap(),
        //    font,
        //    theme: &user_config.theme,
        //    temple_aetherius_society_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_buddhist_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_christian_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_hindu_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_humanist_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_jain_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_jewish_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_muslim_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_rastafarian_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_rosicrucian_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_scientologist_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_self_realization_fellowship_logo: Self::load_image("lgbtq").unwrap(),
        //    temple_sikh_logo: Self::load_image("lgbtq").unwrap(),
        //}
    }

    fn extract_semantic_map_elements(&self, dir: &Path) -> Result<SemanticMapElements> {
        let input_file_path = dir.join(semantic_map::OUTPUT_FILE_NAME);
        let mut input_file = File::open(input_file_path)?;

        let mut buf_vec: Vec<u8> = Vec::new();
        input_file.read_to_end(&mut buf_vec).expect("Could not read note cache.");

        let elements: SemanticMapElements = unsafe {
            rkyv::from_bytes_unchecked(&buf_vec).expect("Could not deserialize node cache.")
        };
        Ok(elements)
    }

    fn load_image(name: &str) -> Result<OwnedImage> {
        let decoder = png::Decoder::new(
            File::open(format!("resources/images/{}.png", name))?
        );

        let mut reader = decoder.read_info()?;

        let mut buf = vec![0; reader.output_buffer_size()];

        let info = reader.next_frame(&mut buf)?;

        if info.bit_depth != BitDepth::Eight {
            return Err("Unsupported bit depth".into())
        }

        let buf_u32 = match info.color_type {
            ColorType::Rgba => {
                let mut buf_u32 = vec![0_u32; reader.output_buffer_size() / 4];
                for (idx_u32, ptr_u32) in buf_u32.iter_mut().enumerate() {
                    let idx_u8 = 4 * idx_u32;
                    let b = buf[idx_u8] as u32;
                    let g = buf[idx_u8 + 1] as u32;
                    let r = buf[idx_u8 + 2] as u32;
                    let a = buf[idx_u8 + 3] as u32;

                    *ptr_u32 =
                        r
                        + (g << 8)
                        + (b << 16)
                        + (a << 24)
                    ;
                }
                buf_u32
            },
            ColorType::GrayscaleAlpha => {
                let mut buf_u32 = vec![0_u32; reader.output_buffer_size() / 2];
                for (idx_u32, ptr_u32) in buf_u32.iter_mut().enumerate() {
                    let idx_u8 = 2 * idx_u32;
                    let g = buf[idx_u8] as u32;
                    let a = buf[idx_u8 + 1] as u32;

                    *ptr_u32 =
                        g
                        + (g << 8)
                        + (g << 16)
                        + (a << 24)
                    ;
                }
                buf_u32
            },
            _ => todo!(),
        };

        Ok(OwnedImage {
            width: info.width.try_into()?,
            height: info.height.try_into()?,
            data: buf_u32,
        })
    }

    fn draw_area(&self, dt: &mut DrawTarget, area: &Area) {
        for polygon in &area.area_polygons {
            if polygon.len() < 2 {
                return;
            }
            let mut pb = PathBuilder::new();
            let (x0, y0) = self.project_mercantor(&polygon[0]);
            pb.move_to(x0, y0);

            for coords in &polygon[1..] {
                let (x, y) = self.project_mercantor(coords);
                pb.line_to(x, y);
            }
            let raquote_path = pb.finish();

            let draw_options = DrawOptions::new();

            dt.fill(
                &raquote_path,
                match area.area_type {
                    semantic::AreaType::Park => &self.theme.park_color,
                    semantic::AreaType::Water => &self.theme.water_color,
                },
                &draw_options,
            );
        }
    }

    fn draw_landmark(&self, dt: &mut DrawTarget, landmark: &Landmark) {
        let (x_center, y_center) = self.project_mercantor(&landmark.into());
        let width = 58.0;
        let height = 48.0;

        if let semantic::LandmarkType::TubeEmergencyExit = landmark.landmark_type {
            return
        }

        let logo = match landmark.landmark_type {
            semantic::LandmarkType::LgbtqMen => &self.lgbtq_logo,
            semantic::LandmarkType::Lgbtq => &self.lgbtq_logo,
            semantic::LandmarkType::CocktailBar => &self.cocktail_logo,
            semantic::LandmarkType::ClimbingBoulder => &self.climbing_boulder_logo,
            semantic::LandmarkType::ClimbingRope => &self.climbing_rope_logo,
            semantic::LandmarkType::ClimbingOutdoor => &self.climbing_outdoor_logo,
            semantic::LandmarkType::Gym => &self.gym_logo,
            semantic::LandmarkType::Hospital => &self.hospital_logo,
            semantic::LandmarkType::MusicVenue => &self.music_logo,
            semantic::LandmarkType::TempleAetheriusSociety => &self.temple_aetherius_society_logo,
            semantic::LandmarkType::TempleBuddhist => &self.temple_buddhist_logo,
            semantic::LandmarkType::TempleChristian => &self.temple_christian_logo,
            semantic::LandmarkType::TempleHindu => &self.temple_hindu_logo,
            semantic::LandmarkType::TempleHumanist => &self.temple_humanist_logo,
            semantic::LandmarkType::TempleJain => &self.temple_jain_logo,
            semantic::LandmarkType::TempleJewish => &self.temple_jewish_logo,
            semantic::LandmarkType::TempleMuslim => &self.temple_muslim_logo,
            semantic::LandmarkType::TempleRastafarian => &self.temple_rastafarian_logo,
            semantic::LandmarkType::TempleRosicucian => &self.temple_rosicrucian_logo,
            semantic::LandmarkType::TempleScientologist => &self.temple_scientologist_logo,
            semantic::LandmarkType::TempleSelfRealizationFellowship => &self.temple_self_realization_fellowship_logo,
            semantic::LandmarkType::TempleSikh => &self.temple_sikh_logo,
            semantic::LandmarkType::Tree => &self.tree_logo,
            semantic::LandmarkType::TubeEmergencyExit => todo!(),
        };

        let img = Image {
            width: self.underground_logo.width,
            height: self.underground_logo.height,
            data: &logo.data,
        };

        let mut draw_options = DrawOptions::new();
        draw_options.blend_mode = BlendMode::SrcOver;

        // dt.draw_image_at(
        dt.draw_image_with_size_at(
            width,
            height,
            x_center - width / 2.0,
            y_center - height / 2.0,
            &img,
            &draw_options,
        );
    }
}

impl Etl for DrawMapEtl<'_> {
    type Input = SemanticMapElements;

    type Output = Vec<Vec<DrawTarget>>;

    fn etl_name(&self) -> &str {
        ETL_NAME
    }

    fn is_cached(&self, dir: &Path) -> Result<bool> {
        Ok(Self::output_path(dir).exists())
    }

    fn clean(&self, dir: &Path) -> Result<()> {
        fs::remove_file(Self::output_path(dir))?;
        Ok(())
    }

    fn extract(&mut self, dir: &Path) -> Result<Self::Input> {
        self.extract_semantic_map_elements(dir)
    }

    fn transform(&mut self, input: Self::Input) -> Result<Self::Output> {
        let mut dts = Vec::new();
        let cell_size = 4096 * 2;
        for cell_x in (0..self.user_config.width_px).step_by(cell_size) {
            let mut dt_col = Vec::new();
            for cell_y in (0..self.user_config.height_px).step_by(cell_size) {
                let cell_width = cell_size.min((self.user_config.width_px - cell_x).try_into()?);
                let cell_height = cell_size.min((self.user_config.height_px - cell_y).try_into()?);
                let mut dt = DrawTarget::new(
                    cell_width.try_into()?,
                    cell_height.try_into()?,
                );
                self.x_shift = cell_x as f32;
                self.y_shift = cell_y as f32;

                if let Source::Solid(s) = self.theme.background_color {
                    dt.clear(s);
                } else {
                    panic!("All colours are solid sources!");
                }

                for area in &input.areas {
                    self.draw_area(&mut dt, &area);
                }
                for road in &input.roads {
                    self.draw_semantic_path(&mut dt, &road, &PathStyle::Road);
                }
                for rail in &input.rails {
                    self.draw_semantic_path(&mut dt, &rail, &PathStyle::Rail);
                }

                let mut sorted_rails = input.tube_rails.clone();
                sorted_rails.sort_by(
                    |rail_a, rail_b| {
                        if rail_a.line < rail_b.line {
                            Ordering::Less
                        } else if rail_a.line == rail_b.line {
                            Ordering::Equal
                        } else {
                            Ordering::Greater
                        }
                    }
                );

                for rail in sorted_rails {
                    self.draw_tube_rail(&mut dt, &rail);
                }
                for station in &input.underground_stations {
                    self.draw_undergound_station(&mut dt, &station);
                }
                for landmark in &input.landmarks {
                    self.draw_landmark(&mut dt, &landmark);
                }
                dt_col.push(dt);
            }
            dts.push(dt_col);
        }
        Ok(dts)
    }

    fn load(&mut self, dir: &Path, output: Self::Output) -> Result<()> {
        for (x, column) in output.iter().enumerate() {
            for (y, dt) in column.iter().enumerate() {
                let filename = format!("output_x{}_y{}.png", x, y);
                let output_path = dir.join(&filename);
                dt.write_png(&output_path)
                    .map_err(|e| format!("Couldn't write png for x={}, y={}: {}", x, y, e))?;
            }
        }
        Ok(())
    }
}
