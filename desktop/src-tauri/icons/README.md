# App Icons

## Current Status

Placeholder icon files have been created:
- `32x32.png` — small placeholder (32×32)
- `128x128.png` — medium placeholder (128×128)
- `128x128@2x.png` — retina placeholder (256×256)
- `icon.icns` — macOS icon placeholder
- `icon.ico` — Windows icon placeholder

## Generating Production Icons

To generate proper icons from a source image (1024×1024 PNG recommended):

### Using Tauri CLI

```bash
cd desktop
npx tauri icon path/to/source-icon.png
```

This will automatically generate all required icon formats and sizes.

### Using ImageMagick (Manual)

If you have ImageMagick installed:

```bash
convert source.png -resize 32x32 32x32.png
convert source.png -resize 128x128 128x128.png
convert source.png -resize 256x256 128x128@2x.png
```

### Using ImageMagick (macOS .icns)

```bash
# Create .icns from PNG
sips -s format icns source.png -o icon.icns
```

### Using ImageMagick (Windows .ico)

```bash
# Create .ico from PNG
convert source.png -define icon:auto-resize=256,128,96,64,48,32,16 icon.ico
```

## Icon Design

The Hive app uses a bee/honeycomb theme with a honey/amber color scheme:
- Primary color: `#F5A623` (Honey/Amber)
- Recommended: Create a bee or honeycomb icon design at 1024×1024
- Include transparency for a professional appearance
- Ensure the icon is clear at small sizes (32×32)
