use std::error::Error;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use allsorts::binary::read::ReadScope;
use allsorts::error::ParseError;
use allsorts::font::read_cmap_subtable;
use allsorts::font_data::FontData;
use allsorts::gsub::{GlyphOrigin, RawGlyph, RawGlyphFlags};
use allsorts::tables::FontTableProvider;
use allsorts::tables::cmap::{Cmap, CmapSubtable};
use allsorts::tinyvec::tiny_vec;
use allsorts::{subset, tag};

fn main() -> Result<(), Box<dyn Error>> {
    let source = "src/components/icons.rs";
    let input = "assets/SymbolsNerdFont-Regular.ttf";
    let output = "target/generated/SymbolsNerdFont-Regular-Subset.ttf";

    println!("cargo:rerun-if-changed={source}");
    println!("cargo:rerun-if-changed={input}");

    let content = std::fs::read_to_string(source)?;

    let mut unicodes = vec![];
    for cap in content.match_indices("\\u{") {
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
    println!("Subsetting font with {} codepoints", unicodes.len());

    let text = unicodes.join("");
    subset_font(input, &text, output)?;

    Ok(())
}

fn subset_font(input: &str, text: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    let buffer = std::fs::read(input)?;
    let font_file = ReadScope::new(&buffer).read::<FontData>()?;
    let font_provider = font_file.table_provider(0)?;

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
    let mut glyph_ids: Vec<_> = glyphs.iter().map(|g| g.glyph_index).collect();
    glyph_ids.dedup();
    if glyph_ids.is_empty() {
        return Err("no glyphs left in font".into());
    }

    println!("Number of glyphs in subset font: {}", glyph_ids.len());

    let new_font = subset::subset(&font_provider, &glyph_ids)?;

    let output_path = Path::new(output_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

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
        .map(|ch| map_glyph(&cmap_subtable, ch))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(glyphs)
}

fn map_glyph(
    cmap_subtable: &CmapSubtable,
    ch: char,
) -> Result<Option<RawGlyph<()>>, ParseError> {
    if let Some(glyph_index) = cmap_subtable.map_glyph(ch as u32)? {
        Ok(Some(RawGlyph {
            unicodes: tiny_vec![[char; 1] => ch],
            glyph_index,
            liga_component_pos: 0,
            glyph_origin: GlyphOrigin::Char(ch),
            flags: RawGlyphFlags::empty(),
            variation: None,
            extra_data: (),
        }))
    } else {
        Ok(None)
    }
}
