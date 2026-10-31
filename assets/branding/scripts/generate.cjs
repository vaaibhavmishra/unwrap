#!/usr/bin/env node
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const sharp = require('sharp');
const { createCanvas, GlobalFonts, SvgExportFlag } = require('@napi-rs/canvas');

const root = path.resolve(__dirname, '..');
const repo = path.resolve(root, '../..');
const inventory = [];
const palette = {
  background: '#0B0D0E', emerald: '#22D982', deepEmerald: '#0EA362',
  mint: '#A7F3D0', lightSurface: '#F5F5F5', secondary: '#8A8F98', darkSurface: '#15191A',
};
const master = fs.readFileSync(path.join(root, 'unwrap-symbol.svg'), 'utf8');
const defs = master.match(/<defs>([\s\S]*?)<\/defs>/)[1];
const geometry = id => master.match(new RegExp(`<path id="${id}" d="([^"]+)"`))[1];
const shape = geometry('silhouette');
const pearl = geometry('pearl-surface');
const fold = geometry('inner-fold');
const svg = (body, width = 512, height = width) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" fill="none">${body}</svg>\n`;
const halo = '<radialGradient id="halo"><stop stop-color="#22D982" stop-opacity=".21"/><stop offset=".55" stop-color="#22D982" stop-opacity=".07"/><stop offset="1" stop-color="#22D982" stop-opacity="0"/></radialGradient>';

function symbol(variant = 'color') {
  if (variant === 'black' || variant === 'white' || variant === 'mono') {
    return `<path d="${shape}" fill="${variant === 'white' ? '#FFFFFF' : variant === 'black' ? '#000000' : 'currentColor'}"/>`;
  }
  if (variant === 'outline') return `<path d="${shape}" stroke="#F5F5F5" stroke-width="14" stroke-linejoin="round"/>`;
  if (variant === 'flat' || variant === 'small') {
    return `<path d="${shape}" fill="${variant === 'small' ? '#0D9F5F' : '#22D982'}"/>` + (variant === 'flat' ? `<path d="${pearl}" fill="#E0F5EB"/>` : '');
  }
  const gradients = variant === 'light' ? defs.replaceAll('#0EA362', '#075F3C').replaceAll('#22D982', '#098350').replaceAll('#55EBA1', '#0D9F5F').replaceAll('#F5F5F5', '#1B7451').replaceAll('#E0F5EB', '#226B50').replaceAll('#A7F3D0', '#0E5739') : defs;
  return `<defs>${gradients}</defs><path d="${shape}" fill="url(#emerald)"/><path d="${pearl}" fill="url(#pearl)"/><path d="${fold}" fill="url(#fold)"/>`;
}

function glowSymbol() {
  return `<defs>${halo}</defs><ellipse cx="294" cy="338" rx="204" ry="168" fill="url(#halo)"/>${symbol()}`;
}

function icon(variant = 'glow') {
  const light = variant === 'light';
  const surface = light ? ['#F5F5F5', '#E0E7E3'] : ['#1C2321', '#0B0D0E'];
  const isFlat = ['flat', 'mono', 'outline'].includes(variant);
  const tile = 'M144 32H368C446 32 480 66 480 144V368C480 446 446 480 368 480H144C66 480 32 446 32 368V144C32 66 66 32 144 32Z';
  return svg(`<defs><linearGradient id="tile" x2="0" y2="1"><stop stop-color="${surface[0]}"/><stop offset="1" stop-color="${surface[1]}"/></linearGradient>${halo}</defs><path d="${tile}" fill="${isFlat ? '#15191A' : 'url(#tile)'}"/>${isFlat || light || variant === 'dark' ? '' : '<ellipse cx="285" cy="341" rx="178" ry="130" fill="url(#halo)"/>'}<g transform="translate(25.6 16) scale(.9)">${symbol(variant === 'mono' ? 'white' : light ? 'light' : variant === 'outline' || variant === 'flat' ? variant : 'color')}</g>`);
}

function write(name, data) {
  const destination = path.join(root, name);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  fs.writeFileSync(destination, data);
  inventory.push(name);
}

async function render(name, content, size, height = size) {
  const data = await sharp(Buffer.from(content), { density: 288 })
    .resize(size, height).toColourspace('srgb').ensureAlpha().png().toBuffer();
  write(name, data);
}

function wordmark(color) {
  const canvas = createCanvas(400, 112, SvgExportFlag.ConvertTextToPaths);
  const ctx = canvas.getContext('2d');
  ctx.font = '92px "Unwrap Inter"';
  ctx.fillStyle = color;
  ctx.letterSpacing = '-3px';
  ctx.textAlign = 'center';
  ctx.fillText('unwrap', 200, 81);
  const content = canvas.getContent().toString();
  if (content.includes('<text')) throw new Error('Wordmark must be converted to editable vector paths');
  return content.replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>[\s\S]*$/, '');
}

function lockup(variant, stacked = false) {
  const textColor = ['light', 'black'].includes(variant) ? '#0B0D0E' : '#F5F5F5';
  const s = symbol(variant === 'black' || variant === 'white' ? variant : variant === 'light' ? 'light' : 'color');
  const w = wordmark(variant === 'black' ? '#000000' : variant === 'white' ? '#FFFFFF' : textColor);
  return stacked
    ? svg(`<g transform="translate(64 0) scale(.75)">${s}</g><g transform="translate(56 382)">${w}</g>`, 512, 520)
    : svg(`<g transform="translate(0 0) scale(.375)">${s}</g><g transform="translate(160 40)">${w}</g>`, 560, 192);
}

async function main() {
  if (!GlobalFonts.registerFromPath(path.join(root, 'source/fonts/Inter-Regular.ttf'), 'Unwrap Inter')) throw new Error('Inter could not be loaded');
  inventory.push('unwrap-symbol.svg');
  write('tokens.json', JSON.stringify(palette, null, 2) + '\n');
  write('tokens.css', `:root {\n${Object.entries(palette).map(([key, value]) => `  --unwrap-${key.replace(/[A-Z]/g, c => '-' + c.toLowerCase())}: ${value};`).join('\n')}\n}\n`);
  const symbols = { glow: glowSymbol(), mono: symbol('mono'), black: symbol('black'), white: symbol('white'), flat: symbol('flat'), light: symbol('light'), outline: symbol('outline'), small: symbol('small') };
  for (const [variant, body] of Object.entries(symbols)) write(`unwrap-symbol-${variant}.svg`, svg(body));
  for (const variant of ['dark', 'light']) write(`unwrap-wordmark-${variant}.svg`, svg(wordmark(variant === 'dark' ? '#F5F5F5' : '#0B0D0E'), 400, 112));
  for (const variant of ['dark', 'light', 'black', 'white']) {
    write(`unwrap-logo-lockup-${variant}.svg`, lockup(variant));
    write(`unwrap-logo-stacked-${variant}.svg`, lockup(variant, true));
    await render(`unwrap-logo-lockup-${variant}.png`, lockup(variant), 1120, 384);
    await render(`unwrap-logo-stacked-${variant}.png`, lockup(variant, true), 1024, 1040);
  }
  write('unwrap-logo-lockup.svg', lockup('dark'));
  await render('unwrap-symbol.png', master, 1024);
  await render('unwrap-symbol-glow.png', svg(glowSymbol()), 1024);
  for (const variant of ['dark', 'light', 'glow', 'flat', 'mono', 'outline']) {
    write(`source/unwrap-app-icon-${variant}.svg`, icon(variant));
    await render(`unwrap-app-icon-${variant}.png`, icon(variant), 1024);
  }
  write('unwrap-app-icon-1024.png', fs.readFileSync(path.join(root, 'unwrap-app-icon-glow.png')));
  for (const size of [16, 32, 48, 64, 128, 256, 512]) {
    // At favicon scale, color facets disappear; a single solid silhouette preserves the opening.
    const content = size <= 32 ? svg(`<g transform="translate(-20.48 -20.48) scale(1.08)">${symbol('small')}</g>`) : icon('glow');
    await render(`unwrap-icon-${size}.png`, content, size);
  }
  write('favicon.svg', svg(`<g transform="translate(-20.48 -20.48) scale(1.08)">${symbol('small')}</g>`));
  for (const size of [16, 32]) write(`favicon-${size}.png`, fs.readFileSync(path.join(root, `unwrap-icon-${size}.png`)));
  // iOS supplies the rounded mask; the touch icon fills its square and has no baked-in frame.
  await render('apple-touch-icon.png', svg(`<rect width="512" height="512" fill="#0B0D0E"/>${glowSymbol()}`), 180);
  for (const size of [16, 32, 128, 256, 512]) {
    for (const scale of [1, 2]) {
      const pixels = size * scale;
      const content = pixels <= 32 ? icon('flat') : icon('glow');
      await render(`unwrap.iconset/icon_${size}x${size}${scale === 2 ? '@2x' : ''}.png`, content, pixels);
    }
  }
  const representations = {
    icp4: '16x16', ic11: '16x16@2x', icp5: '32x32', ic12: '32x32@2x',
    ic07: '128x128', ic13: '128x128@2x', ic08: '256x256', ic14: '256x256@2x',
    ic09: '512x512', ic10: '512x512@2x',
  };
  const chunks = Object.entries(representations).map(([type, name]) => {
    const png = fs.readFileSync(path.join(root, `unwrap.iconset/icon_${name}.png`));
    const header = Buffer.alloc(8); header.write(type); header.writeUInt32BE(png.length + 8, 4);
    return Buffer.concat([header, png]);
  });
  const icnsHeader = Buffer.alloc(8); icnsHeader.write('icns');
  icnsHeader.writeUInt32BE(8 + chunks.reduce((sum, chunk) => sum + chunk.length, 0), 4);
  write('unwrap.icns', Buffer.concat([icnsHeader, ...chunks]));
  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  const entries = [];
  let offset = 6 + 16 * icoSizes.length;
  for (const size of icoSizes) {
    const png = await sharp(Buffer.from(icon(size <= 32 ? 'flat' : 'glow')), { density: 288 }).resize(size, size).ensureAlpha().png().toBuffer();
    const entry = Buffer.alloc(16);
    entry[0] = entry[1] = size === 256 ? 0 : size;
    entry.writeUInt16LE(1, 4); entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(png.length, 8); entry.writeUInt32LE(offset, 12);
    entries.push({ entry, png }); offset += png.length;
  }
  const header = Buffer.alloc(6); header.writeUInt16LE(1, 2); header.writeUInt16LE(entries.length, 4);
  write('unwrap.ico', Buffer.concat([header, ...entries.map(e => e.entry), ...entries.map(e => e.png)]));
  if (process.argv.includes('--integrate')) {
    const mappings = {
      'unwrap.icns': 'app/channels/oss/icon/unwrap.icns',
      'unwrap.ico': 'app/channels/oss/icon/no-padding/icon.ico',
      'unwrap-symbol.svg': 'app/assets/bundled/svg/warp-logo-light.svg',
      'unwrap-symbol-light.svg': 'app/assets/bundled/svg/warp-logo-dark.svg',
      'unwrap-symbol-mono.svg': 'app/assets/bundled/svg/warp-logo-neutral.svg',
      'unwrap-logo-lockup-dark.svg': 'app/assets/bundled/svg/warp-logo-with-light-title.svg',
      'unwrap-logo-lockup-light.svg': 'app/assets/bundled/svg/warp-logo-with-dark-title.svg',
    };
    for (const size of [16, 32, 48, 64, 128, 256, 512]) mappings[`unwrap-icon-${size}.png`] = `app/channels/oss/icon/no-padding/${size}x${size}.png`;
    for (const [source, destination] of Object.entries(mappings)) {
      fs.mkdirSync(path.dirname(path.join(repo, destination)), { recursive: true });
      fs.copyFileSync(path.join(root, source), path.join(repo, destination));
    }
    write('integration.json', JSON.stringify(mappings, null, 2) + '\n');
  }
  const files = inventory.sort().map(name => {
    const bytes = fs.readFileSync(path.join(root, name));
    return { file: name, bytes: bytes.length, sha256: crypto.createHash('sha256').update(bytes).digest('hex') };
  });
  write('manifest.json', JSON.stringify({ name: 'Unwrap', identity: 'V7 Glow', files }, null, 2) + '\n');
  const allFiles = fs.readdirSync(root, { recursive: true }).filter(name => fs.statSync(path.join(root, name)).isFile() && name !== 'INVENTORY.md').sort();
  write('INVENTORY.md', '# Unwrap file inventory\n\nAll paths are relative to `assets/branding/`.\n\n' + allFiles.map(name => `- [${name}](${name})`).join('\n') + '\n- [INVENTORY.md](INVENTORY.md)\n');
  console.log(`Generated ${files.length} assets in ${root}`);
}

main().catch(error => { console.error(error); process.exitCode = 1; });
