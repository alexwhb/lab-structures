#!/usr/bin/env node
// Golden data for `sp_engine::materials` (SPEC MT-2, ENG-5): runs the Materials mockup's own
// `class Component` logic (docs/design/mockup/Materials.dc.html) in Node and records what the
// screen displays after scripted interactions.
//
//   node crates/sp-engine/tests/materials_goldens.mjs           writes materials_goldens.json
//   node crates/sp-engine/tests/materials_goldens.mjs --stdout  prints it instead
//
// Node 22+, no dependencies. Nothing here re-implements the mockup: the logic class is read
// verbatim and run on a tiny stand-in for the mockup runtime (`DCLogic`: `props`, `state`,
// synchronous `setState`). The handlers the screen wires (`pick`, `setTemp`, `setE`, zoom
// `pick`, `toggleDrive`, `newSample`, `reset`) are called from `renderVals()` exactly as the
// markup would, and `tick()` is called as the 50 ms interval would.
//
// Output: `scenarios` (each a list of steps from the default state) and `snaps` (one per line:
// the scenario, the step index after which it was taken, the raw state and the displayed
// values of `renderVals()` — strings verbatim, SVG paths as the mockup builds them — that
// changed since the scenario's previous snapshot). The Rust
// test `materials_goldens.rs` replays the steps on `Bench` and compares; `cargo test` needs no
// Node (the JSON is checked in), and a freshness test re-runs this script when Node exists.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../..');

function load(name) {
  const html = readFileSync(join(ROOT, 'docs/design/mockup', `${name}.dc.html`), 'utf8');
  const m = html.match(/<script type="text\/x-dc"[^>]*>([\s\S]*?)<\/script>/);
  if (!m) throw new Error(`no Component in ${name}`);
  class DCLogic {
    constructor(props) { this.props = props ?? {}; }
    setState(s) { this.state = Object.assign({}, this.state, typeof s === 'function' ? s(this.state) : s); }
  }
  // eslint-disable-next-line no-new-func
  return new Function('DCLogic', `${m[1]}\nreturn Component;`)(DCLogic);
}

const Materials = load('Materials');
const KEYS = ['steel', 'alu', 'concrete', 'timber', 'cf'];

/** Applies one step to a component (the same vocabulary the Rust replay uses). */
function apply(c, step) {
  const [op, arg] = step;
  const v = () => c.renderVals();
  switch (op) {
    case 'pick': v().mats[KEYS.indexOf(arg)].pick(); break;
    case 'temp': v().setTemp({ target: { value: String(arg) } }); break;
    case 'strain': v().setE({ target: { value: String(arg) } }); break;
    case 'zoom': v().zooms[arg === 25 ? 0 : 1].pick(); break;
    case 'toggle': v().toggleDrive(); break;
    case 'fresh': v().newSample(); break;
    case 'reset': v().reset(); break;
    case 'tick': for (let i = 0; i < arg; i++) c.tick(); break;
    case 'snap': break;
    default: throw new Error(`unknown step ${op}`);
  }
}

// Static or derivable values not worth repeating in every snapshot.
const SKIP = new Set(['stageCls', 'notPlaying', 'curveColor', 'specFill']);

/** Everything `renderVals()` displays, without the handlers. */
function snapshot(c) {
  const r = c.renderVals();
  const keep = {};
  for (const [k, val] of Object.entries(r)) {
    if (typeof val === 'function' || SKIP.has(k)) continue;
    if (k === 'mats' || k === 'zooms') { keep[k] = val.map((o) => o.pressed === 'true'); continue; }
    if (Array.isArray(val)) keep[k] = val.map((o) => (o && typeof o === 'object')
      ? Object.fromEntries(Object.entries(o).filter(([, x]) => typeof x !== 'function')) : o);
    else keep[k] = val;
  }
  // The stretch accumulates float steps: write it as a string (shortest round-trip digits) so
  // the Rust side can parse it exactly (serde_json's default float parsing is not exact).
  keep.e = String(keep.e);
  return { state: { ...c.state, e: String(c.state.e), broke: { ...c.state.broke } }, view: keep };
}

const scenarios = [];
const snaps = [];
// Within a scenario each snapshot lists only the view fields that changed since the previous
// one (the first lists them all); the reader merges them in order.
let prevView = null;
function delta(view) {
  const out = {};
  for (const [k, v] of Object.entries(view)) {
    if (!prevView || JSON.stringify(prevView[k]) !== JSON.stringify(v)) out[k] = v;
  }
  prevView = view;
  return out;
}

/** Runs `steps` on a fresh component; `snap` steps record a snapshot. `pullTo` steps (a
 *  generator) are expanded here into `tick` + `snap` steps while the sample is being pulled. */
function run(name, steps) {
  const c = new Materials({});
  const out = [];
  prevView = null;
  const record = () => {
    const { state, view } = snapshot(c);
    snaps.push({ scenario: scenarios.length, step: out.length - 1, state, view: delta(view) });
  };
  for (const step of steps) {
    if (step[0] === 'pull') {
      // ['pull', snapsPerRun]: Pull, then tick in equal chunks until it stops, snapping each.
      const per = step[1];
      out.push(['toggle']); apply(c, ['toggle']);
      const s = c.state;
      const m = c.defs()[s.mat];
      const brk = m.pts[m.pts.length - 1][0];
      const total = Math.ceil(Math.min(brk + 0.1, s.zoom) / (s.zoom === 25 ? 0.08 : 0.006));
      const chunk = Math.max(1, Math.ceil(total / per));
      let guard = 0;
      while (c.state.playing && guard++ < 1000) {
        out.push(['tick', chunk]); apply(c, ['tick', chunk]);
        out.push(['snap']); record();
      }
      continue;
    }
    out.push(step); apply(c, step);
    if (step[0] === 'snap') record();
  }
  scenarios.push({ name, steps: out });
}

// 1. The default state (the reference PNG).
run('default', [['snap']]);

// 2. Pull every material to fracture at room temperature, snapping along the way.
for (const k of KEYS) run(`pull ${k} 20C`, [['pick', k], ['snap'], ['pull', 12]]);

// 3. Pull every material hot.
for (const k of KEYS) run(`pull ${k} 450C`, [['pick', k], ['temp', 450], ['pull', 4]]);
run('pull steel 600C', [['temp', 600], ['pull', 4]]);
run('pull timber 250C', [['pick', 'timber'], ['temp', 250], ['pull', 4]]);
run('pull cf 150C', [['pick', 'cf'], ['temp', 150], ['pull', 4]]);

// 4. Materials × temperatures × strains, scrubbed with the Stretch slider (both axes).
const temps = [-40, 20, 150, 200, 250, 300, 350, 450, 600];
const strainsWide = [0.05, 0.1, 1.5, 4.2, 8.5, 11.95, 15.5, 21.35, 22, 23.5, 25];
const strainsClose = [0.005, 0.07, 0.1375, 0.2, 0.33, 0.35, 0.4, 0.67, 0.86, 0.9, 1.5, 2];
let i = 0;
for (const k of KEYS) {
  const own = k === 'steel' || k === 'alu' ? 25 : 2;
  const other = own === 25 ? 2 : 25;
  const ownList = own === 25 ? strainsWide : strainsClose;
  const otherList = other === 25 ? strainsWide : strainsClose;
  const steps = [['pick', k]];
  for (const t of temps) {
    // Two strains on the material's own axis, one on the other axis.
    steps.push(['temp', t]);
    steps.push(['strain', ownList[i % ownList.length]], ['snap']);
    steps.push(['strain', ownList[(i * 5 + 3) % ownList.length]], ['snap']);
    steps.push(['zoom', other], ['strain', otherList[(i * 7 + 1) % otherList.length]], ['snap']);
    steps.push(['zoom', own]);
    i++;
  }
  run(`scrub ${k}`, steps);
}

// 5. Interactions: zoom clipping a stretch, Pull to the axis end without breaking, restart
//    after fracture, Fresh sample, Pause, break all five, Reset.
run('steel close axis to the end', [['zoom', 2], ['pull', 3], ['toggle'], ['tick', 3], ['snap']]);
run('zoom clips stretch', [['strain', 12.5], ['snap'], ['zoom', 2], ['snap'], ['zoom', 25], ['snap']]);
run('pause and resume', [['toggle'], ['tick', 40], ['toggle'], ['tick', 10], ['snap'], ['toggle'], ['tick', 5], ['snap']]);
run('restart after fracture', [['pick', 'cf'], ['pull', 2], ['toggle'], ['tick', 1], ['snap']]);
run('fresh sample', [['pick', 'alu'], ['strain', 9], ['snap'], ['fresh'], ['snap']]);
run('break all five then reset', [
  ...KEYS.flatMap((k) => [['pick', k], ['toggle'], ['tick', 400], ['snap']]),
  ['temp', 300], ['snap'], ['reset'], ['snap'],
]);
run('temp while broken', [['pick', 'alu'], ['toggle'], ['tick', 400], ['temp', 250], ['snap'], ['temp', 380], ['snap']]);
run('strain past break by slider', [['pick', 'timber'], ['strain', 1.2], ['snap'], ['toggle'], ['tick', 2], ['snap']]);

const text = '{\n"scenarios": [\n' + scenarios.map((s) => JSON.stringify(s)).join(',\n')
  + '\n],\n"snaps": [\n' + snaps.map((s) => JSON.stringify(s)).join(',\n') + '\n]\n}\n';
if (process.argv.includes('--stdout')) process.stdout.write(text);
else {
  writeFileSync(join(HERE, 'materials_goldens.json'), text);
  console.log(`materials_goldens.json: ${scenarios.length} scenarios, ${snaps.length} snapshots, ${text.length} bytes`);
}
