# Icons Font

ashell uses a subset of [Nerd Fonts](https://www.nerdfonts.com/) and some custom icons for its terminal UI.

This custom font is generated using the `generate_font.sh` script located in the `assets` directory.

This script extract the unicodes used in the ashell icons.rs file and generates a custom font containing only those icons.
Then it merges the custom icons with the nerdfont subset to create a single font file used by ashell.

## Custom Icons creation

I'm not an expert in font creation.
Right now I created a svg file `custom_icon.svg` using [Inkscape](https://inkscape.org/) with icons for peripherals devices like keyboard, mouse, headset, etc. that combines the peripheral icon with the battery icon.

To produce the final ttf file I used [FontForge](https://fontforge.org/) to create a new font, import the svg file change the "dimension".

Element -> Font Info -> General -> Em Size: 2048

Then I exported the font as `custom_icon.ttf`.

## Useful commands

### Generate nerdfont subset example

`pyftsubset SymbolsNerdFont-Regular.ttf --output-file=nerdfonts-subset.ttf --unicodes=U+f030c,U+f037d,U+f007b,U+f007d,U+f0081,U+f0084,U+f02cb --layout-features='*' --glyph-names --symbol-cmap --legacy-cmap --notdef-glyph --recommended-glyphs --name-IDs='*' --name-legacy --drop-tables= --no-hinting`

### Merge nerdfont subset and custom icons

`pyftmerge custom_icon.ttf nerdfonts_subset.ttf`
