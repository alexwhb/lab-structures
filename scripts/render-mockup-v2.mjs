#!/usr/bin/env node
// scripts/render-mockup-v2.mjs — renders the mockup bundles in headless Chrome to reference PNGs.
//
//   node scripts/render-mockup-v2.mjs                 all screens at 1440 and 390 px
//   node scripts/render-mockup-v2.mjs BridgeBuild 1440   one screen, one width
//   SETTLE_MS=14000 SUFFIX=-14s node scripts/render-mockup-v2.mjs BridgeCollapse 1440
//                                                    a later moment of a board that runs on its own
//
// Writes docs/design/screenshots/<Screen>-<width>.png. Uses the Chrome DevTools protocol
// (headless Chrome won't size a window below 500 px), so it needs Node 22+ (global WebSocket)
// and Google Chrome. The screens animate (a few screens animate), so the frames differ run to run; they
// are layout references, not pixel goldens.
import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const CHROME = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
// name → mockup height at 1440 px (from the bundle's board sizes)
const SCREENS = { BridgeBuild: 1500, BridgeStorm: 1500, BridgeCollapse: 1500, BridgeCable: 1500, TowerBuild: 1500, TowerStorm: 1500, TowerQuake: 1500 };
// Mockup bugs fixed before capture (see SPEC "Mockup bugs not to copy"). Every v2 board: the shared
// stylesheet's floating `.chip` (position:absolute; translate(-50%,-130%)) leaks into the chip
// buttons, so the Size / Exaggerate sag / Show forces / Material chips float over the layout.
const CHIP_FIX = '<style>.chip{position:static!important;transform:none!important}</style>';
const FIXES = Object.fromEntries(Object.keys(SCREENS).map((k) => [k, CHIP_FIX]));
const [onlyScreen, onlyWidth] = process.argv.slice(2);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const profile = mkdtempSync(join(tmpdir(), 'sp-render-'));
const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn(CHROME, [
  '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
  '--hide-scrollbars', '--force-device-scale-factor=1', '--allow-file-access-from-files', 'about:blank',
], { stdio: 'ignore' });

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
for (const [name, height] of Object.entries(SCREENS)) {
  if (onlyScreen && onlyScreen !== name) continue;
  for (const width of [1440, 390]) {
    if (onlyWidth && Number(onlyWidth) !== width) continue;
    await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
    await send('Page.navigate', { url: pathToFileURL(join(ROOT, 'docs/design/v2/bundles', `${name}.html`)).href });
    await sleep(Number(process.env.SETTLE_MS ?? 3500)); // unpack the bundle, load fonts (and let auto-running tests play)
    if (FIXES[name]) {
      await send('Runtime.evaluate', { expression: `document.head.insertAdjacentHTML('beforeend', ${JSON.stringify(FIXES[name])})` });
      await sleep(300);
    }
    const { cssContentSize } = await send('Page.getLayoutMetrics');
    const h = width === 1440 ? height : Math.ceil(cssContentSize.height);
    const shot = await send('Page.captureScreenshot', {
      format: 'png', captureBeyondViewport: true, clip: { x: 0, y: 0, width, height: h, scale: 1 },
    });
    const out = join(ROOT, 'docs/design/v2/screenshots', `${name}-${width}${process.env.SUFFIX ?? ''}.png`);
    writeFileSync(out, Buffer.from(shot.data, 'base64'));
    console.log(`${out} (${width}×${h})`);
  }
}
ws.close();
chrome.kill();
await sleep(300);
rmSync(profile, { recursive: true, force: true });
