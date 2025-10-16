# Icons Font

ashell uses a subset of [Nerd Fonts](https://www.nerdfonts.com/)
and some custom icons for its terminal UI.

This custom font is generated using the `generate_ashell_icon.sh`
script located in the `assets` directory.

This script extract the unicodes used in the ashell `icons.rs`
file and generates a custom font containing only those icons.
Then it merges the custom icons with the nerdfont subset to create
a single font file used by ashell.

## Custom Icons creation

I'm not an expert in font creation.
Right now I created svg icons using the svg resources in `raw_custom_icons_resources.svg`
using [Inkscape](https://inkscape.org/) with icons for peripherals devices like keyboard,
mouse, headset, etc. that combines the peripheral icon with the battery icon.

To produce the final ttf file i used [GlyphrStudio](https://www.glyphrstudio.com/app/).
The project file is `custom_icons_project.gs2`.

Then I exported the font as `custom_icon_font.otf`.

The generate_font.sh script converts the otf to ttf using and merge the
custom font with the nerd font subset.

## Useful commands

### Generate nerdfont subset example

```bash
pyftsubset SymbolsNerdFont-Regular.ttf --output-file=nerdfonts-subset.ttf --unicodes=U+f030c,U+f037d,U+f007b,U+f007d,U+f0081,U+f0084,U+f02cb

```

### Merge nerdfont subset and custom icons

`pyftmerge custom_icon_font.ttf nerdfonts_subset.ttf`
