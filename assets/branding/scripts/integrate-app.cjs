// Run after generate.cjs to refresh app assets from the Unwrap brand masters.
const fs = require('node:fs');
const path = require('node:path');
const sharp = require('sharp');

const root = path.resolve(__dirname, '../../..');
const brand = path.join(root, 'assets/branding');
const outputs = [];
const read = name => fs.readFileSync(path.join(brand, name));
const image = (name, x, y, width, height = width) =>
  `<image x="${x}" y="${y}" width="${width}" height="${height}" href="data:image/svg+xml;base64,${read(name).toString('base64')}"/>`;
const svg = (width, height, content) => Buffer.from(
  `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">${content}</svg>`
);
function write(relative, data) {
  const file = path.join(root, relative);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, data);
  outputs.push(relative);
}
async function png(relative, artwork, width, height = width) {
  write(relative, await sharp(artwork).resize(width, height).png().toBuffer());
}
async function bmp(relative, artwork, width, height) {
  const pixels = await sharp(artwork).resize(width, height).flatten({ background: '#0B0D0E' })
    .removeAlpha().raw().toBuffer();
  const stride = Math.ceil(width * 3 / 4) * 4;
  const data = Buffer.alloc(54 + stride * height);
  data.write('BM'); data.writeUInt32LE(data.length, 2); data.writeUInt32LE(54, 10);
  data.writeUInt32LE(40, 14); data.writeInt32LE(width, 18); data.writeInt32LE(height, 22);
  data.writeUInt16LE(1, 26); data.writeUInt16LE(24, 28); data.writeUInt32LE(stride * height, 34);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const source = (y * width + x) * 3;
    const target = 54 + (height - 1 - y) * stride + x * 3;
    data[target] = pixels[source + 2]; data[target + 1] = pixels[source + 1]; data[target + 2] = pixels[source];
  }
  write(relative, data);
}

async function main() {
  const symbol = read('unwrap-symbol-white.svg');
  for (const file of ['unwrap-symbol.svg', 'warp-2.svg']) {
    write(`app/assets/bundled/svg/${file}`, symbol);
  }
  write('script/warp.svg', symbol);
  for (const [target, source] of [['warp.svg', 'book-open.svg'], ['warp-drive.svg', 'book-open.svg'], ['warp-3.svg', 'sparkle.svg']]) {
    write(`app/assets/bundled/svg/${target}`, fs.readFileSync(path.join(root, `app/assets/bundled/svg/${source}`)));
  }
  for (let frame = 0; frame < 12; frame++) {
    const opacity = 0.75 + 0.25 * Math.cos(frame * Math.PI * 2 / 12);
    write(`app/assets/bundled/svg/warp-loading-${frame}.svg`, Buffer.from(symbol.toString()
      .replace('<path ', `<path opacity="${opacity.toFixed(4)}" `)));
  }

  // Legacy stored Dock selections must resolve to Unwrap artwork as well.
  const dock = 'app/DockTilePlugin/Resources';
  for (const filename of new Set([...fs.readdirSync(path.join(root, dock)), 'dark.png', 'light.png'])) {
    if (!filename.endsWith('.png')) continue;
    const variant = filename === 'mono.png' ? 'mono' : ['glass_sky.png', 'light.png'].includes(filename) ? 'light'
      : ['dark.png', 'original.png'].includes(filename) ? 'dark' : 'glow';
    await png(`${dock}/${filename}`, read(`unwrap-app-icon-${variant}.png`), 512);
  }
  for (const channel of ['oss', 'stable', 'preview', 'dev', 'local']) {
    for (const size of [16, 32, 48, 64, 128, 256, 512]) {
      write(`app/channels/${channel}/icon/no-padding/${size}x${size}.png`, read(`unwrap-icon-${size}.png`));
    }
    write(`app/channels/${channel}/icon/no-padding/icon.ico`, read('unwrap.ico'));
    const composer = `app/channels/${channel}/icon/AppIcon.icon`;
    if (!fs.existsSync(path.join(root, composer))) continue;
    for (const filename of fs.readdirSync(path.join(root, composer, 'Assets'))) {
      if (filename === 'Unwrap.svg') continue;
      if (filename.endsWith('.svg')) write(`${composer}/Assets/${filename}`, read('unwrap-symbol.svg'));
      if (filename.endsWith('.png')) await png(`${composer}/Assets/${filename}`, read('unwrap-symbol.svg'), 1024);
    }
    write(`${composer}/Assets/Unwrap.svg`, read('unwrap-symbol.svg'));
    write(`${composer}/icon.json`, JSON.stringify({
      fill: { 'automatic-gradient': 'extended-srgb:0.04314,0.05098,0.05490,1.00000' },
      groups: [{ layers: [{ 'image-name': 'Unwrap.svg', name: 'Unwrap', 'blend-mode': 'normal', glass: false }] }],
      'supported-platforms': { circles: ['watchOS'], squares: 'shared' },
    }, null, 2) + '\n');
  }

  const dmg = svg(700, 500, `<rect width="700" height="500" fill="#0B0D0E"/>
    ${image('unwrap-logo-lockup-dark.svg', 240, 35, 220, 80)}
    <path d="M310 250H390M378 238L390 250 378 262" fill="none" stroke="#A7F3D0" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
    <text x="350" y="405" text-anchor="middle" fill="#F5F5F5" font-family="Arial,sans-serif" font-size="17">Drag Unwrap to Applications</text>`);
  await png('app/assets/resources/mac/warp_install_image.png', dmg, 700, 500);
  await bmp('script/windows/installer-images/warp-logo.bmp', svg(58, 58,
    `<rect width="58" height="58" fill="#0B0D0E"/>${image('unwrap-symbol.svg', 0, 0, 58)}`), 58, 58);
  await bmp('script/windows/installer-images/warp-banner.bmp', svg(202, 385,
    `<rect width="202" height="385" fill="#0B0D0E"/>${image('unwrap-logo-stacked-dark.svg', 20, 100, 162, 175)}`), 202, 385);
  await png('app/assets/async/png/onboarding/openwarp_launch_banner.png', svg(420, 92,
    `<rect width="420" height="92" rx="10" fill="#0B0D0E"/>${image('unwrap-logo-lockup-dark.svg', 120, 10, 180, 72)}`), 420, 92);

  // Onboarding uses illustrations so labels remain reproducible alongside the source assets.
  for (const intention of ['agent_intention', 'terminal_intention']) {
    for (const layout of ['horizontal', 'vertical']) {
      const prefix = intention === 'terminal_intention' ? 'terminal_' : '';
      const artwork = svg(1803, 2040, `<rect width="1803" height="2040" fill="#0B0D0E"/>
        ${image('unwrap-logo-lockup-dark.svg', 120, 100, 610, 200)}
        <rect x="110" y="410" width="1583" height="1400" rx="48" fill="#15191A" stroke="#39443E" stroke-width="3"/>
        <g font-family="Arial,sans-serif" fill="#F5F5F5"><text x="210" y="575" font-size="78">Library</text>
        <text x="210" y="685" font-size="37" fill="#A7F3D0">Keep useful work close at hand.</text>
        ${['Workflows', 'Notebooks', 'Prompts', 'Environment variables'].map((name, i) =>
          `<rect x="200" y="${790 + i * 220}" width="1403" height="160" rx="22" fill="#222A26"/><text x="270" y="${890 + i * 220}" font-size="49">${name}</text>`).join('')}
        <text x="210" y="1740" font-size="34" fill="#8A8F98">Saved locally. Ready when you need them.</text></g>`);
      await png(`app/assets/async/png/onboarding/${intention}/${prefix}customize_warpdrive_${layout}.png`, artwork, 1803, 2040);
    }
  }
  const assetCount = outputs.length;
  write('assets/branding/app-integration.json', JSON.stringify({ outputs }, null, 2) + '\n');
  console.log(`Updated ${assetCount} app assets.`);
}
main().catch(error => { console.error(error); process.exitCode = 1; });
