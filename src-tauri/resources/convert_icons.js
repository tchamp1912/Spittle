const fs = require('fs');
const path = require('path');

// List of SVG files to convert
const svgFiles = [
  'tray_idle.svg',
  'tray_idle_dark.svg', 
  'tray_recording.svg',
  'tray_recording_dark.svg',
  'tray_transcribing.svg',
  'tray_transcribing_dark.svg'
];

console.log('To convert SVG files to PNG, use one of these tools:');
console.log('');
console.log('1. ImageMagick (if installed):');
console.log('   convert -density 200 -background none tray_idle.svg tray_idle.png');
console.log('');
console.log('2. Inkscape (if installed):');
console.log('   inkscape -w 128 -h 128 tray_idle.svg -o tray_idle.png');
console.log('');
console.log('3. Online converter:');
console.log('   Visit https://convertio.co/svg-png/');
console.log('');
console.log('SVG files created:');
svgFiles.forEach(file => {
  if (fs.existsSync(file)) {
    const size = fs.statSync(file).size;
    console.log(`  ✓ ${file} (${size} bytes)`);
  }
});
