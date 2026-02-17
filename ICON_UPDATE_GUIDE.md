# Spittle Icon Update Guide

This guide explains how to update all platform-specific icons to use the new Spittle droplet branding.

## Current Status

✅ **React Components**:

- `src/components/icons/SpittleIcon.tsx` - Updated with 💧 emoji
- `src/components/icons/SpittleTextLogo.tsx` - SVG logo with Spittle text

✅ **SVG Logo Files** (in `src-tauri/icons/`):

- `spittle-logo.svg` - Full logo with text (new)
- `spittle-simple-icon.svg` - Simple droplet icon (new)
- `spittle-icon.svg` - Existing SVG (can be updated)

⚠️ **Platform Icon Files** (need updating):
These binary icon files still have Spittle branding and need to be regenerated:

### macOS Icons

- `icon.icns` - macOS app icon (ICNS format)

### Windows Icons

- `icon.ico` - Windows app icon (ICO format)
- `Square30x30Logo.png` through `StoreLogo.png` - Windows Store tiles

### Linux Icons

- `32x32.png`, `64x64.png`, `128x128.png`, `128x128@2x.png`, `256x256.png` - App icons
- `icon.png` - Generic icon

### Android Icons (if needed in future)

- `android/mipmap-*/ic_launcher.png` - Various resolutions
- `android/mipmap-*/ic_launcher_foreground.png`
- `android/mipmap-*/ic_launcher_round.png`

### iOS Icons (if needed in future)

- `ios/AppIcon-*.png` - Various resolutions

## How to Update Icons

### Option 1: Using Tauri CLI (Recommended)

The Tauri CLI can auto-generate icons from a source image:

1. **Prepare a source image**:
   - Use `spittle-simple-icon.svg` as the base
   - Convert to PNG if needed (512x512 or higher recommended)
   - Ensure it has proper padding and aspect ratio

2. **Use Tauri to regenerate icons**:

   ```bash
   cd src-tauri
   tauri icon <path-to-source-image>
   ```

3. **Verify the icons** are updated in `icons/` directory

### Option 2: Using Image Conversion Tools

If Tauri CLI doesn't work for your setup:

1. **Convert SVG to PNG**:

   ```bash
   # Using ImageMagick
   convert -density 300 src-tauri/icons/spittle-simple-icon.svg -resize 512x512 src-tauri/icons/icon-512.png

   # Or using Inkscape
   inkscape --export-width=512 --export-height=512 src-tauri/icons/spittle-simple-icon.svg -o src-tauri/icons/icon-512.png
   ```

2. **Generate platform-specific icons**:
   - **macOS** (.icns): Use `png2icns` or online converters
   - **Windows** (.ico): Use ImageMagick or online converters
   - **Linux** (.png): Create multiple sizes (32, 64, 128, 256)
   - **Android**: Create multiple resolutions (ldpi, mdpi, hdpi, xhdpi, xxhdpi, xxxhdpi)
   - **iOS**: Create required sizes (20, 29, 40, 60, 76, 83.5, 1024)

### Option 3: Using Online Icon Generators

1. Visit an online icon generator (e.g., https://www.convertio.co/png-icns/)
2. Upload `spittle-simple-icon.svg` (converted to PNG first if needed)
3. Generate icons for each platform
4. Download and replace the files in `src-tauri/icons/`

## Icon Design Guidelines

For consistency with the Spittle brand:

- **Color**: Use sky blue (#0EA5E9) as primary color
- **Shape**: Water droplet (💧) shape
- **Style**: Modern, clean, rounded corners
- **Padding**: At least 10% margin from edges
- **Background**: Transparent for most platforms, rounded square for iOS/Android

## Source Files Available

- `src-tauri/icons/spittle-logo.svg` - Full logo with text
- `src-tauri/icons/spittle-simple-icon.svg` - Icon only (recommended for app icon)
- `src-tauri/icons/spittle-icon.svg` - Alternative SVG format

## Testing Icons

After updating icons:

1. **Development**: Run `bun run tauri dev` and verify tray icon appears correctly
2. **Build**: Run `bun run tauri build` and check bundled app icon
3. **macOS**: Check icon in Finder and Dock
4. **Windows**: Check icon in Start Menu and taskbar
5. **Linux**: Check icon in application menu

## Tools Reference

### Icon Generation Tools

- **Tauri CLI**: Built-in icon generator (recommended)
- **ImageMagick**: `convert` command for format conversion
- **Inkscape**: Vector to raster conversion
- **Online**: Convertio, CloudConvert, Ezgif for quick conversions

### Icon Format References

- **macOS**: ICNS format (multiple resolutions in one file)
- **Windows**: ICO format (multiple resolutions)
- **Linux**: PNG format (separate files for each size)
- **Android**: PNG format (separate folders per DPI)
- **iOS**: PNG format (separate files per size)

## Next Steps

1. [ ] Choose icon generation method
2. [ ] Generate platform-specific icons
3. [ ] Replace files in `src-tauri/icons/`
4. [ ] Test icons in development and production builds
5. [ ] Verify appearance on each platform
