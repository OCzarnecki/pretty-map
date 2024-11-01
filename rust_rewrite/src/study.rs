use std::{fs::File, usize};

use png::{BitDepth, ColorType};
use raqote::*;
use sw_composite::*;

use crate::{
    errors::Result,
};

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

pub struct OwnedImage {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u32>,
}

pub fn draw_image_raw(dt: &mut DrawTarget, img: &OwnedImage, x: i32, y: i32) {
    let buffer_width = dt.width();
    let buffer_height = dt.height();
    let buffer = dt.get_data_u8_mut();

    for current_img_x in 0..img.width {
        for current_img_y in 0..img.height {
            let buffer_x = current_img_x + x;
            let buffer_y = current_img_y + y;
            if buffer_x >= buffer_width {
                continue;
            }
            if buffer_y >= buffer_height {
                continue;
            }
            let idx_buffer = buffer_y * buffer_width + buffer_x;
            let idx_img = current_img_y * img.width + current_img_x;

            let new_r = img.data[idx_img as usize] & 0x000000FF;
            let new_g = img.data[idx_img as usize] & 0x000000FF;
            let new_b = img.data[idx_img as usize] & 0x000000FF;
            let new_a = img.data[idx_img as usize] & 0x000000FF;

            let old_r = buffer[4 * idx_buffer as usize] as u32;
            let old_g = buffer[4 * idx_buffer as usize + 1] as u32;
            let old_b = buffer[4 * idx_buffer as usize + 2] as u32;
            let old_a = buffer[4 * idx_buffer as usize + 3] as u32;

            buffer[4 * idx_buffer as usize] = ((new_r * new_a + old_r * (255 - new_a)) / 255) as u8;
            buffer[4 * idx_buffer as usize + 1] = ((new_g * new_a + old_g * (255 - new_a)) / 255) as u8;
            buffer[4 * idx_buffer as usize + 2] = ((new_b * new_a + old_b * (255 - new_a)) / 255) as u8;
            buffer[4 * idx_buffer as usize + 3] = 0xFF;
        }
    }
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
        png::ColorType::Rgba => {
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

pub fn big_image() {
    let width = 23622;
    let height = 17717;

    let mut dt = DrawTarget::new(width, height);
    dt.clear(SolidSource::from_unpremultiplied_argb(255, 255, 0, 0));

    let image = load_image("crucifix-out").expect("God is dead.");

    let mut draw_options = DrawOptions::new();
    draw_options.blend_mode = BlendMode::Overlay;

    let img = Image {
        width: image.width,
        height: image.height,
        data: &image.data,
    };

    let step_size = 300;
    for x in tqdm::tqdm((0..width).step_by(step_size)) {
        for y in (0..height).step_by(step_size) {
            // dt.draw_image_at(x as f32, y as f32, &img, &draw_options);
            draw_image_raw(&mut dt, &image, x, y);
        }
    }

    dt.write_png("study_out.png").expect("Couldn't write.");
}
