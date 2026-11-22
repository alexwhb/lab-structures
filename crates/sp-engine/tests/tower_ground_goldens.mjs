#!/usr/bin/env node
// Golden data for `sp_engine::tower` and `sp_engine::ground`: runs the mockups' own
// `class Component` logic (docs/design/mockup/Tower.dc.html, Ground.dc.html) in Node for a set
// of states and writes what the screens display (tiles, status, sentences, goals, chips) plus the
// raw `calc()` numbers.
//
//   node crates/sp-engine/tests/tower_ground_goldens.mjs
//
// Writes crates/sp-engine/tests/tower_goldens.json and ground_goldens.json (checked in; the Rust
// tests `tower_goldens.rs` / `ground_goldens.rs` read them, so `cargo test` needs no Node).
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../..');

function load(name) {
  const html = readFileSync(join(ROOT, 'docs/design/mockup', `${name}.dc.html`), 'utf8');
  const m = html.match(/<script type="text\/x-dc"[^>]*>([\s\S]*?)<\/script>/);
  if (!m) throw new Error(`no Component in ${name}`);
  class DCLogic { constructor(props) { this.props = props; } setState(s) { Object.assign(this.state, s); } }
  // eslint-disable-next-line no-new-func
  return new Function('DCLogic', `${m[1]}\nreturn Component;`)(DCLogic);
}

// A small deterministic LCG so the sample is stable run to run.
let seed = 12345;
const rnd = () => { seed = (seed * 1103515245 + 12345) % 2147483648; return seed / 2147483648; };
const pick = (xs) => xs[Math.floor(rnd() * xs.length)];
// One state per line (small, diffable).
const lines = (xs) => '[\n' + xs.map((x) => JSON.stringify(x)).join(',\n') + '\n]\n';

// ---------------------------------------------------------------------------------------------
// Tower

const Tower = load('Tower');
const towerStates = [];
const tDefault = { floors: 60, sys: 'core', shape: 'straight', damper: false, v10: 30, ex: 40, playing: false, t: 0 };
towerStates.push({ ...tDefault });
for (const sys of ['core', 'tube', 'outrigger', 'braced'])
  for (const shape of ['straight', 'tapered', 'twisted', 'hollow'])
    towerStates.push({ ...tDefault, sys, shape });
for (const floors of [5, 6, 30, 59, 61, 99, 100, 101, 119, 120]) towerStates.push({ ...tDefault, floors });
for (const v10 of [5, 6, 29, 31, 50]) towerStates.push({ ...tDefault, v10 });
towerStates.push({ ...tDefault, damper: true });
towerStates.push({ ...tDefault, floors: 100, sys: 'braced', shape: 'twisted', damper: true });
towerStates.push({ ...tDefault, floors: 100, sys: 'braced', shape: 'tapered', damper: true });
towerStates.push({ ...tDefault, floors: 60, sys: 'braced', shape: 'straight' });
while (towerStates.length < 110) {
  towerStates.push({
    ...tDefault,
    floors: pick([5, 8, 12, 20, 25, 33, 40, 47, 52, 60, 66, 75, 80, 88, 95, 100, 110, 120]),
    sys: pick(['core', 'tube', 'outrigger', 'braced']),
    shape: pick(['straight', 'tapered', 'twisted', 'hollow']),
    damper: rnd() < 0.5,
    v10: pick([5, 10, 15, 20, 25, 30, 35, 40, 45, 50]),
  });
}

const tower = towerStates.map((st) => {
  const c = new Tower({});
  Object.assign(c.state, st);
  const v = c.renderVals();
  const r = c.calc(st.floors, st.sys, st.shape, st.v10, st.damper);
  return {
    state: { floors: st.floors, sys: st.sys, shape: st.shape, damper: st.damper, v10: st.v10 },
    calc: { H: r.H, V: r.V, M: r.M, delta: r.delta, f1: r.f1, mg: r.mg, vH: r.vH, fs: r.fs, ratio: r.ratio, lock: r.lock, near: r.near, cost: r.cost, dyn: r.dyn },
    tiles: v.tiles.map((t) => ({ k: t.k, v: t.v, n: t.n, c: t.c })),
    status: v.statusLabel, statusColor: v.statusColor,
    accel: v.accel, comfortPos: v.comfortPos, comfortText: v.comfortText, comfortColor: v.comfortColor,
    compare: v.compare.map((x) => ({ name: x.name, v: x.v, w: x.w, color: x.color, tc: x.tc })),
    goals: v.goals.map((g) => ({ text: g.text, done: g.done })),
    explain: v.explain, swayChip: v.swayChip, vTop: v.vTop, v10Label: v.v10Label, clock: v.clock,
    stageLabel: v.stageLabel, height: v.height,
  };
});
writeFileSync(join(HERE, 'tower_goldens.json'), lines(tower));

// ---------------------------------------------------------------------------------------------
// Ground

const Ground = load('Ground');
const gDefault = { soil: 'soft', found: 'mat', floors: 6, pileL: 34, quake: false, t: 10, playing: false, pisaTried: false };
const groundStates = [{ ...gDefault }];
for (const soil of ['dense', 'stiff', 'soft', 'loose'])
  for (const found of ['footing', 'mat', 'piles', 'caisson'])
    groundStates.push({ ...gDefault, soil, found });
// The scenarios (after one 60 ms tick and after 30 years).
const scen = [
  { soil: 'soft', found: 'mat', floors: 6, quake: false, t: 0, pisaTried: true },
  { soil: 'loose', found: 'mat', floors: 12, quake: true, t: 0 },
  { soil: 'stiff', found: 'piles', pileL: 34, floors: 60, quake: false, t: 0 },
  { soil: 'soft', found: 'piles', pileL: 20, floors: 10, quake: false, t: 0 },
];
for (const s of scen) for (const t of [0, 0.25, 30]) groundStates.push({ ...gDefault, ...s, t });
// Pisa fixed with piles / caissons / mat (goal 3 both ways).
for (const found of ['footing', 'mat', 'piles', 'caisson'])
  for (const pileL of [8, 20, 30, 40])
    groundStates.push({ ...gDefault, found, pileL, t: 30, pisaTried: true });
groundStates.push({ ...gDefault, floors: 5, pisaTried: true, found: 'caisson' });
groundStates.push({ ...gDefault, soil: 'loose', quake: true, found: 'piles', pileL: 29, t: 5 });
groundStates.push({ ...gDefault, soil: 'loose', quake: true, found: 'piles', pileL: 30, t: 5 });
while (groundStates.length < 130) {
  groundStates.push({
    ...gDefault,
    soil: pick(['dense', 'stiff', 'soft', 'loose']),
    found: pick(['footing', 'mat', 'piles', 'caisson']),
    floors: pick([2, 3, 6, 9, 12, 18, 25, 33, 40, 50, 60, 70, 80]),
    pileL: pick([8, 12, 16, 20, 25, 29, 30, 34, 40]),
    quake: rnd() < 0.3,
    t: pick([0, 0.25, 0.5, 1, 2.5, 5, 10, 15.5, 20, 29.75, 30]),
    pisaTried: rnd() < 0.3,
  });
}

const ground = groundStates.map((st) => {
  const c = new Ground({});
  Object.assign(c.state, st);
  const v = c.renderVals();
  const r = c.calc(st);
  return {
    state: { soil: st.soil, found: st.found, floors: st.floors, pileL: st.pileL, quake: st.quake, t: st.t, pisaTried: st.pisaTried },
    calc: { load: r.load, q: r.q, cap: r.cap, FS: r.FS, deep: r.deep, toRock: r.toRock, liquefied: r.liquefied, settleF: r.settleF, tiltF: r.tiltF, settle: r.settle, tilt: r.tilt, tau: r.tau },
    tiles: v.tiles.map((t) => ({ k: t.k, v: t.v, n: t.n, c: t.c })),
    status: v.statusLabel, statusColor: v.statusColor,
    fsText: v.fsText, fsColor: v.fsColor, fsPos: v.fsPos,
    goals: v.goals.map((g) => ({ text: g.text, done: g.done })),
    explain: v.explain, tiltNow: v.tiltNow, settleNow: v.settleNow, yearLabel: v.yearLabel, clock: v.clock,
    pileLabel: v.pileLabel, stageLabel: v.stageLabel, soilName: v.soilName, wet: v.wet,
  };
});
writeFileSync(join(HERE, 'ground_goldens.json'), lines(ground));
console.log(`tower: ${tower.length} states, ground: ${ground.length} states`);
