use std::{fs::{self, File}, io::Read, path::{Path, PathBuf}};

use png::{self, BitDepth};
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
    pub road_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub rail_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub text_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub water_color: Source<'a>,

    #[serde(deserialize_with = "deserialize")]
    pub park_color: Source<'a>,
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
        (x as f32, y as f32)
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
        let semantic_path = &tube_rail.path;
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
            &Self::stroke(10.0),
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
        dt.draw_glyphs(&self.font, point_size, &ids, &positions, &source, &options);
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

        // dt.draw_image_at(
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
            lgbtq_logo: Self::load_image("lgbtq").unwrap(),
            cocktail_logo: Self::load_image("cocktail").unwrap(),
            font,
            theme: &user_config.theme,
        }
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
        let mut buf_u32 = vec![0_u32; reader.output_buffer_size() / 4];

        let info = reader.next_frame(&mut buf)?;

        if info.bit_depth != BitDepth::Eight {
            return Err("Unsupported bit depth".into())
        }

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
        let width = 32.0;
        let height = 32.0;

        if let semantic::LandmarkType::Tree = landmark.landmark_type {
            return
        }

        let logo = match landmark.landmark_type {
            semantic::LandmarkType::LgbtqMen => &self.lgbtq_logo,
            semantic::LandmarkType::Lgbtq => &self.lgbtq_logo,
            semantic::LandmarkType::CocktailBar => &self.cocktail_logo,
            semantic::LandmarkType::Hospital => &self.cocktail_logo,
            //semantic::LandmarkType::Tree => &self.cocktail_logo,
            _ => todo!(),
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

    type Output = DrawTarget;

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
        let mut dt = DrawTarget::new(
            self.user_config.width_px.try_into()?,
            self.user_config.height_px.try_into()?
        );

        dt.clear(SolidSource::from_unpremultiplied_argb(
            0xff, 0xff, 0xff, 0xff,
        ));

        for area in input.areas {
            self.draw_area(&mut dt, &area);
        }
        for road in input.roads {
            self.draw_semantic_path(&mut dt, &road, &PathStyle::Road);
        }
        for rail in input.rails {
            self.draw_semantic_path(&mut dt, &rail, &PathStyle::Rail);
        }
        for rail in input.tube_rails {
            self.draw_tube_rail(&mut dt, &rail);
        }
        for station in input.underground_stations {
            self.draw_undergound_station(&mut dt, &station);
        }
        for landmark in input.landmarks {
            self.draw_landmark(&mut dt, &landmark);
        }
        Ok(dt)
    }

    fn load(&mut self, dir: &Path, output: Self::Output) -> Result<()> {
        output.write_png(
            Self::output_path(dir)
        ).map_err(|_| "Couldn't write png. (encoding error)".into())
    }
}
