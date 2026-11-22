#!/usr/bin/env node
// assets/icon/render-icon.mjs — renders the app icon (assets/icon/structures-icon.svg) to every
// macOS .iconset size in headless Chrome and packs "Structures Playground.icns" with iconutil.
//
//   node assets/icon/render-icon.mjs                 → assets/icon/Structures Playground.icns
//   node assets/icon/render-icon.mjs --iconset DIR   … and keep the PNGs in DIR (an .iconset)
//
// Each size is rasterised from the vector at its own pixel size (no downscaling), on a
// transparent background. Uses the Chrome DevTools protocol like scripts/render-mockup.mjs
// (Node 22+ for the global WebSocket, Google Chrome or $CHROME), then `sips` checks every PNG's
// size and alpha and `iconutil` builds the .icns. macOS only (sips, iconutil). The .icns is
// committed, so scripts/bundle-macos.sh needs neither Node nor Chrome; rerun this after editing
// the SVG.
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const SVG = join(HERE, 'structures-icon.svg');
const ICNS = join(HERE, 'Structures Playground.icns');
const CHROME = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
// icon_<n>x<n>[@2x].png → pixel size, as iconutil expects
const SIZES = [
  ['icon_16x16.png', 16], ['icon_16x16@2x.png', 32],
  ['icon_32x32.png', 32], ['icon_32x32@2x.png', 64],
  ['icon_128x128.png', 128], ['icon_128x128@2x.png', 256],
  ['icon_256x256.png', 256], ['icon_256x256@2x.png', 512],
  ['icon_512x512.png', 512], ['icon_512x512@2x.png', 1024],
];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const args = process.argv.slice(2);
let keep = null;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--iconset') keep = resolve(args[++i] ?? '');
  else if (args[i] === '-h' || args[i] === '--help') {
    console.log(readFileSync(fileURLToPath(import.meta.url), 'utf8').split('\n').slice(1, 13).join('\n'));
    process.exit(0);
  } else { console.error(`unknown argument ${args[i]}`); process.exit(2); }
}

const work = mkdtempSync(join(tmpdir(), 'sp-icon-'));
const iconset = keep ?? join(work, 'Structures Playground.iconset');
mkdirSync(iconset, { recursive: true });
const svg = readFileSync(SVG, 'utf8');

const port = 9800 + Math.floor(Math.random() * 500);
const chrome = spawn(CHROME, [
  '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${join(work, 'profile')}`,
  '--hide-scrollbars', '--force-device-scale-factor=1', 'about:blank',
], { stdio: 'ignore' });
chrome.on('error', (e) => { console.error(`render-icon: cannot start Chrome (${CHROME}): ${e.message}`); process.exit(1); });

async function target() {
  for (let i = 0; i < 100; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const page = list.find((t) => t.type === 'page');
      if (page) return page.webSocketDebuggerUrl;
    } catch { /* not up yet */ }
    await sleep(100);
  }
  throw new Error('Chrome did not start');
}

try {
  const ws = new WebSocket(await target());
  await new Promise((r) => ws.addEventListener('open', r, { once: true }));
  let nextId = 1;
  const pending = new Map();
  ws.addEventListener('message', (e) => {
    const m = JSON.parse(e.data);
    if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
  });
  const send = (method, params = {}) => new Promise((res, rej) => {
    const id = nextId++;
    pending.set(id, (m) => (m.error ? rej(new Error(`${method}: ${m.error.message}`)) : res(m.result)));
    ws.send(JSON.stringify({ id, method, params }));
  });

  await send('Page.enable');
  await send('Emulation.setDefaultBackgroundColorOverride', { color: { r: 0, g: 0, b: 0, a: 0 } });
  const rendered = new Map(); // px → PNG bytes (16@2x and 32 share a size, and so on)
  for (const [name, px] of SIZES) {
    if (!rendered.has(px)) {
      await send('Emulation.setDeviceMetricsOverride', { width: px, height: px, deviceScaleFactor: 1, mobile: false });
      const sized = svg.replace(/width="1024" height="1024"/, `width="${px}" height="${px}"`);
      const html = `<!doctype html><html><head><style>html,body{margin:0;padding:0;background:transparent;overflow:hidden}svg{display:block}</style></head><body>${sized}</body></html>`;
      await send('Page.navigate', { url: `data:text/html;base64,${Buffer.from(html).toString('base64')}` });
      await sleep(250);
      const shot = await send('Page.captureScreenshot', {
        format: 'png', clip: { x: 0, y: 0, width: px, height: px, scale: 1 },
      });
      rendered.set(px, Buffer.from(shot.data, 'base64'));
    }
    writeFileSync(join(iconset, name), rendered.get(px));
  }
  ws.close();

  for (const [name, px] of SIZES) {
    const info = execFileSync('sips', ['-g', 'pixelWidth', '-g', 'pixelHeight', '-g', 'hasAlpha', join(iconset, name)], { encoding: 'utf8' });
    const w = Number(/pixelWidth: (\d+)/.exec(info)?.[1]);
    const h = Number(/pixelHeight: (\d+)/.exec(info)?.[1]);
    if (w !== px || h !== px || !/hasAlpha: yes/.test(info)) {
      throw new Error(`${name}: ${w}×${h}, ${/hasAlpha: (\w+)/.exec(info)?.[1]} alpha (want ${px}×${px} with alpha)`);
    }
  }
  execFileSync('iconutil', ['-c', 'icns', iconset, '-o', ICNS]);
  console.log(`${SIZES.length} sizes → ${ICNS}${keep ? ` (PNGs in ${keep})` : ''}`);
} finally {
  chrome.kill();
  await sleep(300);
  rmSync(work, { recursive: true, force: true });
}
