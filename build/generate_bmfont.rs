//! Generates a BMFont (FNT and PNG) from a source font file (TTF/OTF).
//!
//! Usage:
//!   bazel run @rules_ciq//tools:generate_bmfont -- <font> <output> <height> [--chars <chars>] [--anti-alias] [--reference-chars <reference_chars>] [--weight <weight>]
//!
//! This tool rasterizes a font at a specified height and packs the glyphs into a texture.
//! It produces two files:
//! - `<output>.fnt`: The font descriptor file.
//! - `<output>.png`: The font texture.
//!
//! Arguments:
//! - `font`: Path to the source font file.
//! - `output`: Output path prefix (e.g., "myfont" produces "myfont.fnt" and "myfont.png").
//! - `height`: Font height in pixels.
//! - `chars` (optional): Characters to include in the font. Defaults to a standard ASCII set.
//! - `anti_alias` (optional): Enable anti-aliasing.
//! - `reference_chars` (optional): If specified, the font scale is adjusted so that the
//!   vertical span of these characters exactly matches the requested `height`.
//! - `weight` (optional): Font weight for variable fonts (e.g. 100 to 900).

use clap::Parser;
use image::{ImageBuffer, Rgba};
use rectangle_pack::{
    contains_smallest_box, pack_rects, volume_heuristic, GroupedRectsToPlace, RectToInsert,
    TargetBin,
};
use swash::FontRef;
use swash::scale::{ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the source font file (TTF/OTF)
    font: PathBuf,

    /// Output path prefix (e.g. "myfont" will produce "myfont.fnt" and "myfont.png")
    output: PathBuf,

    /// Font height in pixels
    height: u32,

    /// Characters to include in the font
    #[arg(
        short,
        long,
        default_value = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"
    )]
    chars: String,

    /// Enable anti-aliasing
    #[arg(long)]
    anti_alias: bool,

    /// Reference characters for scaling (optional).
    /// If specified, the font will be scaled such that the vertical span of
    /// this string characters equals the requested height.
    #[arg(long)]
    reference_chars: Option<String>,

    /// Font weight for variable fonts (optional).
    #[arg(long)]
    weight: Option<u16>,
}

struct CharInfo {
    id: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    xoffset: i32,
    yoffset: i32,
    xadvance: i32,
}

struct RenderedGlyph {
    c: char,
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    advance: f32,
    data: Vec<u8>,
}

fn calculate_scale(
    font: &FontRef,
    height: u32,
    weight: Option<u16>,
    reference_chars: Option<&str>,
    context: &mut ScaleContext,
) -> f32 {
    let default_size = height as f32;

    let chars = match reference_chars {
        Some(s) if !s.is_empty() => s,
        _ => return default_size,
    };

    let charmap = font.charmap();
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    let mut found = false;

    let mut builder = context.builder(*font).size(default_size).hint(true);
    if let Some(w) = weight {
        builder = builder.variations(Some(swash::Setting::from(("wght", w as f32))));
    }
    let mut scaler = builder.build();
    let mut render = swash::scale::Render::new(&[
        Source::ColorOutline(0),
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::Outline,
    ]);

    for c in chars.chars() {
        let glyph_id = charmap.map(c);
        if glyph_id != 0 {
            if let Some(image) = render.format(Format::Alpha).render(&mut scaler, glyph_id) {
                let p = image.placement;
                let top = p.top as f32;
                let bottom = (p.top - p.height as i32) as f32;
                if bottom < min_y {
                    min_y = bottom;
                }
                if top > max_y {
                    max_y = top;
                }
                found = true;
            }
        }
    }

    if found {
        let content_height = max_y - min_y;
        if content_height > 0.0 {
            let scale_factor = default_size / content_height;
            return default_size * scale_factor;
        }
    }

    default_size
}

fn get_unique_chars(chars: &str) -> Vec<char> {
    let mut unique: Vec<char> = chars.chars().collect();
    unique.push(' ');
    unique.sort();
    unique.dedup();
    unique
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Load font
    let font_data = std::fs::read(&args.font)?;
    let font = FontRef::from_index(&font_data, 0).ok_or("Error constructing font")?;

    let mut context = ScaleContext::new();

    // Calculate scale
    let scale_size = calculate_scale(
        &font,
        args.height,
        args.weight,
        args.reference_chars.as_deref(),
        &mut context,
    );

    // Build final scaler
    let mut builder = context.builder(font).size(scale_size).hint(true);
    if let Some(w) = args.weight {
        builder = builder.variations(Some(swash::Setting::from(("wght", w as f32))));
    }
    let mut scaler = builder.build();

    // Rasterize glyphs
    let metrics = font.metrics(&[]);
    let upm = metrics.units_per_em as f32;
    let metrics_scale = scale_size / upm;
    let ascent = (metrics.ascent * metrics_scale).ceil() as i32;

    let mut glyphs = Vec::new();
    let unique_chars = get_unique_chars(&args.chars);
    let charmap = font.charmap();

    let mut render = swash::scale::Render::new(&[
        Source::ColorOutline(0),
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::Outline,
    ]);

    for c in unique_chars {
        let glyph_id = charmap.map(c);
        let advance = font.glyph_metrics(&[]).advance_width(glyph_id) * metrics_scale;
        
        if glyph_id != 0 {
            if let Some(image) = render.format(Format::Alpha).render(&mut scaler, glyph_id) {
                glyphs.push(RenderedGlyph {
                    c,
                    width: image.placement.width,
                    height: image.placement.height,
                    left: image.placement.left,
                    top: image.placement.top,
                    advance,
                    data: image.data,
                });
            } else {
                glyphs.push(RenderedGlyph {
                    c,
                    width: 0,
                    height: 0,
                    left: 0,
                    top: 0,
                    advance,
                    data: Vec::new(),
                });
            }
        } else {
            // Space or invisible character or missing
            glyphs.push(RenderedGlyph {
                c,
                width: 0,
                height: 0,
                left: 0,
                top: 0,
                advance,
                data: Vec::new(),
            });
        }
    }

    // Prepare rects for packing
    let mut rects: GroupedRectsToPlace<usize, ()> = GroupedRectsToPlace::new();
    let padding = 1;

    for (i, glyph) in glyphs.iter().enumerate() {
        // Dimensions we need to pack (ensure at least 1x1)
        let w = (glyph.width + padding).max(1);
        let h = (glyph.height + padding).max(1);

        rects.push_rect(i, None, RectToInsert::new(w, h, 1));
    }

    let mut width = 256;
    let mut height = 256;
    let packed_locations;

    loop {
        let mut target_bins = BTreeMap::new();
        target_bins.insert(0, TargetBin::new(width, height, 1));

        let placement_analysis = pack_rects(
            &rects,
            &mut target_bins,
            &volume_heuristic,
            &contains_smallest_box,
        );

        if let Ok(analysis) = placement_analysis {
            // Check if all rects were packed
            if analysis.packed_locations().len() == glyphs.len() {
                packed_locations = analysis.packed_locations().clone();
                break;
            }
        }
        // If error or incomplete packing, grow
        if width <= height {
            width += 256;
        } else {
            height += 256;
        }

        if width > 8192 || height > 8192 {
            return Err("Texture size too large".into());
        }
    }

    // Create texture
    let mut texture = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 255]));
    let mut char_data = Vec::with_capacity(glyphs.len());

    // Iterate sorted by original index (which maps to 'i' in packed_locations)
    for i in 0..glyphs.len() {
        if let Some((_, rect)) = packed_locations.get(&i) {
            let glyph = &glyphs[i];

            let target_x = rect.x();
            let target_y = rect.y();

            if glyph.width > 0 && glyph.height > 0 {
                let mut data_idx = 0;
                for y in 0..glyph.height {
                    for x in 0..glyph.width {
                        let v = glyph.data[data_idx];
                        let alpha = if args.anti_alias {
                            v
                        } else {
                            if v > 127 {
                                255
                            } else {
                                0
                            }
                        };

                        if alpha > 0 {
                            let px = target_x + x;
                            let py = target_y + y;
                            if px < width && py < height {
                                texture.put_pixel(px, py, Rgba([alpha, alpha, alpha, 255]));
                            }
                        }
                        data_idx += 1;
                    }
                }
            }

            char_data.push(CharInfo {
                id: glyph.c as u32,
                x: target_x,
                y: target_y,
                width: glyph.width,
                height: glyph.height,
                xoffset: glyph.left,
                yoffset: ascent - glyph.top,
                xadvance: glyph.advance.round() as i32,
            });
        }
    }

    char_data.sort_by_key(|cd| cd.id);

    // Check for potential rendering issues
    for char_info in &char_data {
        if char_info.xoffset < 0 {
            eprintln!(
                "Warning: Glyph '{}' (id={}) has negative xoffset ({}), which may result in truncation on the left side of the character.",
                std::char::from_u32(char_info.id).unwrap_or('?'),
                char_info.id,
                char_info.xoffset
            );
        }
        if char_info.xoffset + char_info.xadvance < char_info.width as i32 {
            eprintln!(
                "Warning: Glyph '{}' (id={}) has xoffset ({}) + xadvance ({}) < width ({}), which may result in truncation on the right side of the character.",
                std::char::from_u32(char_info.id).unwrap_or('?'),
                char_info.id,
                char_info.xoffset,
                char_info.xadvance,
                char_info.width
            );
        }
    }

    // Save image
    let mut image_path = args.output.clone();
    image_path.set_extension("png");
    texture.save(&image_path)?;

    // Save FNT
    let mut fnt_path = args.output.clone();
    fnt_path.set_extension("fnt");
    let file = File::create(&fnt_path)?;
    let mut writer = BufWriter::new(file);

    let font_name = "Custom";
    let texture_width = width;
    let texture_height = height;

    writeln!(
        writer,
        "info face=\"{}\" size={} bold=0 italic=0 charset=\"\" unicode=1 stretchH=100 smooth=1 aa={} padding=0,0,0,0 spacing=1,1 outline=0",
        font_name,
        args.height,
        if args.anti_alias { 1 } else { 0 }
    )?;

    let descent = (metrics.descent * metrics_scale).floor() as i32;
    let line_gap = (metrics.leading * metrics_scale).ceil() as i32;
    // Swash descent is a positive distance scalar, unlike rusttype
    let line_height = ascent + descent + line_gap;

    writeln!(
        writer,
        "common lineHeight={} base={} scaleW={} scaleH={} pages=1 packed=0 alphaChnl=0 redChnl=1 greenChnl=1 blueChnl=1",
        line_height, ascent, texture_width, texture_height
    )?;

    writeln!(
        writer,
        "page id=0 file=\"{}\"",
        image_path.file_name().unwrap().to_string_lossy()
    )?;
    writeln!(writer, "chars count={}", char_data.len())?;

    for char_info in char_data {
        writeln!(
            writer,
            "char id={} x={} y={} width={} height={} xoffset={} yoffset={} xadvance={} page=0 chnl=15",
            char_info.id,
            char_info.x,
            char_info.y,
            char_info.width,
            char_info.height,
            char_info.xoffset,
            char_info.yoffset,
            char_info.xadvance
        )?;
    }

    Ok(())
}
