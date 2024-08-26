use std::usize;

use raqote::*;

mod fk {
    pub use font_kit::canvas::{Canvas, Format, RasterizationOptions};
    pub use font_kit::font::Font;
    pub use font_kit::hinting::HintingOptions;
    pub use pathfinder_geometry::transform2d::Transform2F;
    pub use pathfinder_geometry::vector::{vec2f, vec2i};
}

pub fn set_px(data: &mut [u32], width: i32, height: i32, x: i32, y: i32, color: u32) {
    debug_assert!(0 <= x && x < width && 0 <= y && y < height);
    data[usize::try_from(width * y + x).unwrap()] = color;
}

pub fn draw_text(
    self_: &mut DrawTarget,
    font: &fk::Font,
    point_size: f32,
    text: &str,
    start: Point,
    src: &Source,
    options: &DrawOptions,
) {
    let mut start = fk::vec2f(start.x, start.y);
    let mut ids = Vec::new();
    let mut positions = Vec::new();
    for c in text.chars() {
        let id = font.glyph_for_char(c).unwrap();
        ids.push(id);
        positions.push(Point::new(start.x(), start.y()));
        start += font.advance(id).unwrap() * point_size / 24. / 96. * 2.0;
    }
    self_.draw_glyphs(font, point_size, &ids, &positions, src, options);
}

pub fn run() {
    const WIDTH: usize = 500;
    const HEIGHT: usize = 500;
    let mut data = [0u32; WIDTH * HEIGHT];

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let color = if ((2 * x) & y) % 3 == 0 { 0xFFFF0000 } else { 0xFFFF00FF };
            set_px(
                &mut data,
                WIDTH.try_into().unwrap(),
                HEIGHT.try_into().unwrap(),
                x.try_into().unwrap(),
                y.try_into().unwrap(),
                color,
            );
        }
    }

    let img = Image{
        width: WIDTH.try_into().unwrap(),
        height: HEIGHT.try_into().unwrap(),
        data: &data,
    };

    let mut dt = DrawTarget::new(WIDTH.try_into().unwrap(), HEIGHT.try_into().unwrap());

    dt.draw_image_at(0.0, 0.0, &img, &DrawOptions::new());
    let font = font_kit::loader::Loader::from_file(
        &mut std::fs::File::open("resources/fonts/Domine-Bold.ttf").unwrap(), 0
        //&mut std::fs::File::open("/usr/share/fonts/noto/NotoSansMono-Light.ttf").unwrap(), 0
    ).unwrap();
    draw_text(
        &mut dt,
        &font,
        80.0,
        "Marylebone",
        Point::new(250.0, 150.0),
        &Source::Solid(SolidSource::from_unpremultiplied_argb(255, 0, 0, 0)),
        &DrawOptions::new()
    );
    draw_text(
        &mut dt,
        &font,
        40.0,
        "Marylebone",
        Point::new(250.0, 250.0),
        &Source::Solid(SolidSource::from_unpremultiplied_argb(255, 0, 0, 0)),
        &DrawOptions::new()
    );
    draw_text(
        &mut dt,
        &font,
        20.0,
        "Marylebone",
        Point::new(250.0, 300.0),
        &Source::Solid(SolidSource::from_unpremultiplied_argb(255, 0, 0, 0)),
        &DrawOptions::new()
    );
    dt.write_png("test.png").expect("Could not write test.png");
}


