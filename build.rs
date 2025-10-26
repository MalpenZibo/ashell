use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::str;

use allsorts::binary::read::ReadScope;
use allsorts::error::ParseError;
use allsorts::font::read_cmap_subtable;
use allsorts::font_data::FontData;
use allsorts::gsub::{GlyphOrigin, RawGlyph, RawGlyphFlags};
use allsorts::tables::FontTableProvider;
use allsorts::tables::cmap::Cmap;
use allsorts::tables::cmap::CmapSubtable;
use allsorts::tinyvec::tiny_vec;
use allsorts::unicode::VariationSelector;
use allsorts::{subset, tag};

pub fn main() -> Result<(), Box<dyn Error>> {
    let source = "src/components/icons.rs";
    let input = "assets/SymbolsNerdFont-Regular.ttf";
    let input_mono = "assets/SymbolsNerdFontMono-Regular.ttf";

    let output = "target/generated/SymbolsNerdFont-Regular-Subset.ttf";
    let output_mono = "target/generated/SymbolsNerdFontMono-Regular-Subset.ttf";

    let content = std::fs::read_to_string(source)?;

    let mut unicodes = vec![];
    for cap in content.match_indices("\\u{") {
        // find start of \u{XXXX}
        let start = cap.0 + 3;
        let rest = &content[start..];
        if let Some(end) = rest.find('}') {
            let hex = &rest[..end];
            unicodes.push(hex.to_string());
        }
    }
    let unicodes: Vec<String> = unicodes
        .into_iter()
        .map(|h| {
            std::char::from_u32(u32::from_str_radix(&h, 16).unwrap())
                .unwrap()
                .to_string()
        })
        .collect();
    println!("Request the following unicodes {:?}", unicodes);

    let text = unicodes.join("");

    subset_text(input, &text, output)?;
    subset_text(input_mono, &text, output_mono)?;

    Ok(())
}

fn subset_text(input: &str, text: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    let buffer = std::fs::read(input)?;
    let font_file = ReadScope::new(&buffer).read::<FontData>()?;
    let font_provider = font_file.table_provider(0)?;

    // Work out the glyphs we want to keep from the text
    let mut glyphs = chars_to_glyphs(&font_provider, text)?;
    let notdef = RawGlyph {
        unicodes: tiny_vec![],
        glyph_index: 0,
        liga_component_pos: 0,
        glyph_origin: GlyphOrigin::Direct,
        flags: RawGlyphFlags::empty(),
        variation: None,
        extra_data: (),
    };
    glyphs.insert(0, Some(notdef));

    let mut glyphs: Vec<RawGlyph<()>> = glyphs.into_iter().flatten().collect();
    glyphs.sort_by(|a, b| a.glyph_index.cmp(&b.glyph_index));
    let mut glyph_ids = glyphs
        .iter()
        .map(|glyph| glyph.glyph_index)
        .collect::<Vec<_>>();
    glyph_ids.dedup();
    if glyph_ids.is_empty() {
        return Err("no glyphs left in font".to_string().into());
    }

    println!("Number of glyphs in new font: {}", glyph_ids.len());

    // Subset
    let new_font = subset::subset(&font_provider, &glyph_ids)?;

    let output_path = Path::new(output_path);

    // Create all parent directories
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write out the new font
    let mut output = File::create(output_path)?;
    output.write_all(&new_font)?;

    Ok(())
}

fn chars_to_glyphs<F: FontTableProvider>(
    font_provider: &F,
    text: &str,
) -> Result<Vec<Option<RawGlyph<()>>>, Box<dyn Error>> {
    let cmap_data = font_provider.read_table_data(tag::CMAP)?;
    let cmap = ReadScope::new(&cmap_data).read::<Cmap>()?;
    let (_, cmap_subtable) = read_cmap_subtable(&cmap)?.ok_or(Into::<Box<dyn Error>>::into(
        "no suitable cmap sub-table found".to_string(),
    ))?;

    let glyphs = text
        .chars()
        .map(|ch| map(&cmap_subtable, ch, None))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(glyphs)
}

pub(crate) fn map(
    cmap_subtable: &CmapSubtable,
    ch: char,
    variation: Option<VariationSelector>,
) -> Result<Option<RawGlyph<()>>, ParseError> {
    if let Some(glyph_index) = cmap_subtable.map_glyph(ch as u32)? {
        let glyph = make(ch, glyph_index, variation);
        Ok(Some(glyph))
    } else {
        Ok(None)
    }
}

pub(crate) fn make(
    ch: char,
    glyph_index: u16,
    variation: Option<VariationSelector>,
) -> RawGlyph<()> {
    RawGlyph {
        unicodes: tiny_vec![[char; 1] => ch],
        glyph_index,
        liga_component_pos: 0,
        glyph_origin: GlyphOrigin::Char(ch),
        flags: RawGlyphFlags::empty(),
        variation,
        extra_data: (),
    }
}
