#!/usr/bin/env bash
# Extracts all Unicode codepoints from a Rust file defining an Icons enum
# and outputs them as a comma-separated list in the form U+xxxx

# Resolve the folder where this script is located
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
ICONS_FILE="$SCRIPT_DIR/../src/components/icons.rs"
FONT_IN="$SCRIPT_DIR/SymbolsNerdFont-Regular.ttf"
CUSTOM_FONT_IN="$SCRIPT_DIR/custom_icon_font.otf"
CUSTOM_FONT_IN_TTF="$SCRIPT_DIR/custom_icon_font.ttf"
SUBSET_FONT_OUT="$SCRIPT_DIR/nerdfonts_subset.ttf"
FONT_OUT="$SCRIPT_DIR/ashell_icon.ttf"

PYTON_RENAME_SCRIPT="$SCRIPT_DIR/rename_font.py"

# --- Check dependencies ---
if ! command -v pyftsubset &>/dev/null; then
  echo "Error: pyftsubset not found. Install it with 'pip install fonttools'."
  exit 1
fi

# --- Check files ---
if [ ! -f "$ICONS_FILE" ]; then
  echo "Error: icons file not found at $ICONS_FILE"
  exit 1
fi
if [ ! -f "$FONT_IN" ]; then
  echo "Error: font file not found at $FONT_IN"
  exit 1
fi
if [ ! -f "$CUSTOM_FONT_IN" ]; then
  echo "Error: font file not found at $CUSTOM_FONT_IN"
  exit 1
fi

# --- Extract Unicode list ---
UNICODE_LIST="$(
  grep -oP '\\u\{[0-9a-fA-F]+\}' "$ICONS_FILE" |
    sed -E 's/.*\\u\{([0-9a-fA-F]+)\}.*/U+\1/' |
    paste -sd ',' -
)"

echo "Found Unicode list:"
echo "$UNICODE_LIST"
echo

# --- Run pyftsubset ---
echo "Subsetting font..."
pyftsubset "$FONT_IN" \
  --output-file="$SUBSET_FONT_OUT" \
  --unicodes="$UNICODE_LIST" \
  --no-layout-closure

echo "Convert custom icon font to ttf and merge with subset font..."
fontforge -lang=ff -c 'Open($1); Generate($2)' $CUSTOM_FONT_IN $CUSTOM_FONT_IN_TTF
pyftmerge $CUSTOM_FONT_IN_TTF $SUBSET_FONT_OUT --output-file=$FONT_OUT

# python $PYTON_RENAME_SCRIPT
