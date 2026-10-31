const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const sharp = require('sharp');
const { createCanvas, GlobalFonts, SvgExportFlag } = require('@napi-rs/canvas');

const root = path.resolve(__dirname, '..');
const brand = path.resolve(root, '../branding');
const repo = path.resolve(root, '../..');
const sourcePath = 'app/assets/async/png/onboarding/agent_intention/theme/theme_dark_horizontal.png';
const C = { bg: '#0B0D0E', white: '#F5F5F5', green: '#22D982', mint: '#A7F3D0', gray: '#9CA7A1', line: '#28362F' };
const items = [];
let sequence = 0;
GlobalFonts.registerFromPath(path.join(brand, 'source/fonts/Inter-Regular.ttf'), 'Unwrap Inter');

function text(x, y, copy, size = 24, color = C.white, tracking = 0) {
  const canvas = createCanvas(2300, 2100, SvgExportFlag.ConvertTextToPaths);
  const ctx = canvas.getContext('2d');
  ctx.font = `${size}px "Unwrap Inter"`;
  ctx.fillStyle = color;
  ctx.letterSpacing = `${tracking}px`;
  ctx.fillText(copy, x, y);
  return `<g aria-label="${copy.replaceAll('&', '&amp;').replaceAll('"', '&quot;')}">${canvas.getContent().toString().replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>[\s\S]*$/, '')}</g>`;
}

function artwork(file, x, y, w, h) {
  const id = `art${sequence++}_`;
  const content = fs.readFileSync(path.join(brand, file), 'utf8')
    .replaceAll(/id="([^"]+)"/g, (_, name) => `id="${id}${name}"`)
    .replaceAll(/url\(#([^)]+)\)/g, (_, name) => `url(#${id}${name})`)
    .replace(/<svg[^>]*>/, match => match.replace(/width="[^"]+"/, `width="${w}"`).replace(/height="[^"]+"/, `height="${h}"`).replace('<svg ', `<svg x="${x}" y="${y}" `));
  return content;
}

const logo = (x, y, w = 240) => artwork('unwrap-logo-lockup-dark.svg', x, y, w, w * 192 / 560);
const mark = (x, y, s) => artwork('unwrap-symbol.svg', x, y, s, s);
const rect = (x, y, w, h, color, r = 0) => `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${r}" fill="${color}"/>`;
const line = (x, y, w) => `<path d="M${x} ${y}h${w}" stroke="${C.line}"/>`;
function background(w, h, light = false) {
  return rect(0, 0, w, h, light ? '#E0F5EB' : C.bg) + (light ? '' : `<defs><radialGradient id="ambient"><stop stop-color="#155A3A" stop-opacity=".25"/><stop offset="1" stop-color="#155A3A" stop-opacity="0"/></radialGradient></defs><ellipse cx="${w * .82}" cy="${h * .66}" rx="${w * .65}" ry="${h * .75}" fill="url(#ambient)"/>`);
}

function photo(file, x, y, w, h, rounded = 10) {
  const id = `photo${sequence++}`;
  const data = fs.readFileSync(path.join(root, 'source', file)).toString('base64');
  return `<defs><clipPath id="${id}"><rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${rounded}"/></clipPath></defs><image href="data:image/png;base64,${data}" x="${x}" y="${y}" width="${w}" height="${h}" preserveAspectRatio="xMidYMid meet" clip-path="url(#${id})"/>`;
}

function screen(x, y, w, detail = false) {
  const ratio = detail ? 1400 / 474 : 1440 / 918;
  const h = w / ratio;
  return rect(x - 1, y - 1, w + 2, h + 2, '#34443B', 13) + photo(detail ? 'unwrap-code-detail.png' : 'unwrap-ui-preview.png', x, y, w, h, 12);
}

async function save(name, w, h, body, description) {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}"><title>${description}</title>${body}</svg>`;
  fs.writeFileSync(path.join(root, 'source', `${name}.svg`), svg);
  const out = path.join(root, 'exports', `${name}.png`);
  await sharp(Buffer.from(svg)).toColourspace('srgb').flatten({ background: C.bg }).png().toFile(out);
  const meta = await sharp(out).metadata();
  if (meta.width !== w || meta.height !== h) throw new Error(`${name}: wrong dimensions`);
  items.push({ name, width: w, height: h, description, png: `exports/${name}.png`, svg: `source/${name}.svg`, bytes: fs.statSync(out).size, sha256: crypto.createHash('sha256').update(fs.readFileSync(out)).digest('hex') });
}

async function main() {
  for (const dir of ['source', 'exports', 'review']) fs.mkdirSync(path.join(root, dir), { recursive: true });
  const source = path.join(repo, sourcePath);
  fs.copyFileSync(source, path.join(root, 'source/developer-original.png'));
  const base = await sharp(source).extract({ left: 114, top: 0, width: 1440, height: 918 }).flatten({ background: '#080A09' }).png().toBuffer();
  const header = `<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="918">${rect(0, 0, 1440, 44, '#15191A')}<g fill="#49504C"><circle cx="23" cy="22" r="7"/><circle cx="48" cy="22" r="7"/><circle cx="73" cy="22" r="7"/></g>${mark(93, 3, 37)}${text(142, 29, 'unwrap', 20, C.white, -.6)}${text(636, 28, 'Code review', 15, C.gray)}<path d="M0 43.5H1440" stroke="#303730"/></svg>`;
  await sharp(base).composite([{ input: Buffer.from(header) }]).png().toFile(path.join(root, 'source/unwrap-ui-preview.png'));
  const detail = await sharp(source).extract({ left: 140, top: 464, width: 1390, height: 438 }).extend({ top: 31, bottom: 5, left: 5, right: 5, background: '#15191A' }).png().toBuffer();
  const detailHeader = `<svg xmlns="http://www.w3.org/2000/svg" width="1400" height="474">${text(19, 22, 'unwrap / code review', 15, C.gray)}</svg>`;
  await sharp(detail).composite([{ input: Buffer.from(detailHeader) }]).png().toFile(path.join(root, 'source/unwrap-code-detail.png'));

  await save('unwrap-launch-landscape', 1200, 630,
    background(1200, 630) + logo(35, 26, 235) + text(58, 215, 'Your terminal.', 65, C.white, -2.8) + text(58, 290, 'Unwrapped.', 65, C.mint, -2.8) + text(61, 353, 'Open source. Built-in AI.', 25, C.gray, -.5) + text(61, 391, 'Bring your own keys.', 25, C.gray, -.5) + screen(566, 144, 582) + line(60, 549, 1080) + text(60, 586, 'NATIVE TERMINAL', 14, C.gray, 1.6) + text(450, 586, 'YOUR MODELS', 14, C.gray, 1.6) + text(935, 586, 'OPEN SOURCE', 14, C.mint, 1.6),
    'Unwrap launch graphic with rebranded repository UI preview');

  await save('unwrap-product-screenshot', 1920, 1080,
    background(1920, 1080) + logo(90, 30, 260) + text(122, 183, 'From command to code.', 66, C.white, -2.5) + text(1210, 79, 'OPEN SOURCE / BUILT-IN AI', 19, C.gray, 2) + screen(350, 226, 1220) + text(122, 1039, 'YOUR TERMINAL. YOUR MODELS.', 17, C.mint, 2),
    'Branded Unwrap product showcase adapted from the repository screenshot');

  await save('unwrap-square', 1080, 1080,
    background(1080, 1080) + logo(39, 28, 244) + text(66, 226, 'Your terminal.', 83, C.white, -3.7) + text(66, 320, 'Your rules.', 83, C.mint, -3.7) + text(70, 379, 'A minimal terminal with built-in AI agents.', 26, C.gray, -.5) + screen(70, 442, 940, true) + line(70, 851, 940) + text(70, 908, '01', 16, C.green, 1) + text(120, 910, 'Open source', 28, C.white, -.6) + text(572, 908, '02', 16, C.green, 1) + text(622, 910, 'Bring your own keys', 28, C.white, -.6) + text(70, 1006, 'LESS BETWEEN YOU AND YOUR SHELL.', 15, C.gray, 2),
    'Unwrap square social post with actual code-review screenshot detail');

  await save('unwrap-portrait', 1080, 1350,
    background(1080, 1350) + logo(42, 34, 250) + text(72, 244, 'A terminal.', 92, C.white, -4.8) + text(72, 349, 'Unwrapped.', 92, C.mint, -4.8) + text(76, 416, 'Open source. Built-in AI. Your keys.', 28, C.gray, -.5) + screen(76, 496, 928) + line(76, 1160, 928) + text(76, 1216, 'Built in Rust.', 29, C.white, -.6) + text(622, 1216, 'Built for your workflow.', 29, C.white, -.6) + text(76, 1280, 'NATIVE TERMINAL / BRING YOUR OWN MODEL', 15, C.gray, 1.5),
    'Unwrap portrait social post with rebranded repository UI preview');

  await save('unwrap-story', 1080, 1920,
    background(1080, 1920) + logo(58, 252, 300) + text(83, 482, 'Your terminal.', 85, C.white, -4) + text(83, 582, 'Unwrapped.', 85, C.mint, -4) + text(89, 665, 'Open source. Built-in AI. Your keys.', 28, C.gray, -.5) + screen(76, 793, 928) + line(88, 1470, 904) + text(88, 1536, 'Native performance.', 32, C.white, -.6) + text(88, 1591, 'A familiar shell. Room to build.', 32, C.white, -.6) + text(88, 1660, 'MEET UNWRAP', 17, C.mint, 2),
    'Unwrap story post with 250-pixel top and bottom copy safe zones');

  await save('unwrap-banner', 1500, 500,
    background(1500, 500) + mark(1040, 20, 440) + text(390, 192, 'Your terminal.', 71, C.white, -3.5) + text(390, 273, 'Unwrapped.', 71, C.mint, -3.5) + text(395, 331, 'Open source. Built-in AI. Your keys.', 25, C.gray, -.5),
    'Unwrap social profile banner with clear lower-left avatar area');

  await save('unwrap-avatar', 1024, 1024,
    background(1024, 1024) + mark(96, 80, 832),
    'Unwrap profile avatar using the original symbol without wordmark');

  const preview = fs.readFileSync(path.join(root, 'source/unwrap-ui-preview.png'));
  fs.writeFileSync(path.join(root, 'exports/unwrap-ui-preview.png'), preview);
  items.push({ name: 'unwrap-ui-preview', width: 1440, height: 918, description: 'Standalone rebranded repository screenshot composite', png: 'exports/unwrap-ui-preview.png', bytes: preview.length, sha256: crypto.createHash('sha256').update(preview).digest('hex') });
  fs.writeFileSync(path.join(root, 'manifest.json'), JSON.stringify({ brand: 'Unwrap V7 Glow', date: '2026-10-30', sourceScreenshot: sourcePath, screenshotTreatment: 'Existing developer marketing screenshot, cropped to remove reflection; top tab strip replaced by Unwrap mark and title. Agent transcript, command text and code review content remain source pixels. Detail crop preserves original code-editing panel. These are rebranded marketing composites, not live captures of the current Unwrap build.', items }, null, 2) + '\n');
  console.log(JSON.stringify(items.map(({ name, width, height, bytes }) => ({ name, width, height, bytes })), null, 2));
}

main().catch(error => { console.error(error); process.exitCode = 1; });
