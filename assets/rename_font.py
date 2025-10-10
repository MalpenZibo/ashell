from fontTools.ttLib import TTFont
from pathlib import Path

# Path to the font relative to this script
SCRIPT_DIR = Path(__file__).resolve().parent
FONT_PATH = SCRIPT_DIR / "ashell_icon.ttf"  # adjust filename if needed

# Load the font
font = TTFont(FONT_PATH)

# Common name IDs to update
name_map = {
    1: "Ashell Nerd Font",            # Font Family
    2: "Regular",                       # Subfamily
    3: "AshellNerdFont-Regular",      # Unique Identifier
    4: "Ashell Nerd Font Regular",    # Full Font Name
    6: "AshellNerdFont-Regular",      # PostScript Name
}

for record in font["name"].names:
    if record.nameID in name_map:
        new_name = name_map[record.nameID]
        record.string = new_name.encode("utf-16-be")

# --- Save back to same file (overwrite) ---
font.save(FONT_PATH)
font.close()
print("✅ Font renamed and saved as ashell_icon.ttf")
