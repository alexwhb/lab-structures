#!/usr/bin/env node
// scripts/mockup-goldens.mjs — golden data for sp-engine from the mockup's own JavaScript
// (SPEC ENG-5, ADV-10).
//
//   node scripts/mockup-goldens.mjs            writes crates/sp-engine/tests/golden/advanced.json
//
// Node 22+, no dependencies. Nothing here re-implements the mockup: the `class Component`
// script of `docs/design/mockup/Advanced.dc.html` is read verbatim and run on a tiny stand-in
// for the mockup runtime (`DCLogic`: `props`, `state`, synchronous `setState`). For each
// toolbar state we set `state`, call the mockup's own `renderVals()` and record what it would
// show (netlist lines, member table, checks, status, influence-line tiles and path), plus the
// raw numbers of its `solve()` (displacements, member forces, utilisations, mass) written as
// strings in JavaScript's shortest round-trip form, so the Rust side can parse them exactly
// (`str::parse::<f64>` is correctly rounded; serde_json's default float parser is not) and
// compare them bit for bit.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const OUT = join(ROOT, 'crates/sp-engine/tests/golden');

// The mockup runtime, reduced to what the logic class uses.
class DCLogic {
  constructor(props) { this.props = props ?? {}; this.state = {}; }
  setState(patch) { Object.assign(this.state, typeof patch === 'function' ? patch(this.state) : patch); }
}

/** Loads `class Component` from a mockup screen, verbatim. */
function loadComponent(screen) {
  const html = readFileSync(join(ROOT, 'docs/design/mockup', `${screen}.dc.html`), 'utf8');
  const m = html.match(/<script type="text\/x-dc" data-dc-script=""[^>]*>([\s\S]*?)<\/script>/);
  if (!m) throw new Error(`no logic script in ${screen}.dc.html`);
  const src = m[1];
  if (!/class Component extends DCLogic/.test(src)) throw new Error(`no class Component in ${screen}`);
  // eslint-disable-next-line no-new-func
  return new Function('DCLogic', `${src}\nreturn Component;`)(DCLogic);
}

const Advanced = loadComponent('Advanced');
const c = new Advanced({});
const DEFAULTS = { ...c.state };

// Toolbar states: every style × braced × material at the defaults, then sweeps of size, truck,
// x and the influence-line pick per style × braced (deterministic, no randomness).
const styles = ['pratt', 'howe', 'warren'];
const mats = ['steel', 'alu', 'timber', 'cf'];
const ils = ['topmid', 'botmid', 'post', 'vert'];
const sizes = [1, 2, 3, 5, 6, 9, 12, 17, 20, 4];
const trucks = [0, 10, 40, 77, 120, 55, 1, 99, 40, 23];
const xs = [0, 0.5, 7.5, 16, 24, 31.5, 40, 47.5, 48, 12.5];
const states = [];
for (const style of styles) for (const braced of [false, true]) for (const mat of mats) {
  states.push({ ...DEFAULTS, style, braced, mat });
}
let k = 0;
for (const style of styles) for (const braced of [false, true]) {
  for (let i = 0; i < 10; i++, k++) {
    states.push({
      ...DEFAULTS, style, braced,
      mat: mats[(i + k) % 4], size: sizes[i], truckT: trucks[(i * 3 + k) % 10],
      xt: xs[(i * 7 + k) % 10], il: ils[(i + 2 * k) % 4],
    });
  }
}

const cases = states.map((s) => {
  c.state = { ...s };
  const v = c.renderVals();
  // The raw solve the screen is drawn from (same call renderVals makes).
  const mat = c.mats()[s.mat];
  const model = c.build(s.style, s.braced);
  const sol = c.solve(model, mat, s.size, s.truckT * 9.81, s.xt, {});
  return {
    state: { style: s.style, braced: s.braced, mat: s.mat, size: s.size, truckT: s.truckT, xt: s.xt, il: s.il },
    lines: v.lines.map((l) => [l.dir, l.args, l.cmt]),
    ok: sol.ok,
    u: sol.u.map(String),
    N: sol.res.map((r) => String(r.N)),
    ut: sol.res.map((r) => String(r.ut)),
    mass: String(sol.mass),
    rows: v.rows.map((r) => [r.id, r.nodes, r.N, r.sig, r.mode, r.ut, r.c, r.uc]),
    checks: v.checks.map((ch) => [ch.k, ch.v, ch.note, ch.c]),
    memLabels: v.memLabels.map((m) => [m.id, m.c]),
    statusLabel: v.statusLabel,
    statusColor: v.statusColor,
    schemLabel: v.schemLabel,
    clock: v.clock,
    ilId: v.ilId,
    ilNow: v.ilNow,
    ilPeak: v.ilPeak,
    ilPeakAt: v.ilPeakAt,
    ilLim: v.ilLim,
    ilColor: v.ilColor,
    ilLabel: v.ilLabel,
    ilD: v.ilD,
    ilCap: v.ilCap,
    ilCx: v.ilCx,
    ilCy: v.ilCy,
    ilPicks: v.ilPicks.map((p) => p.name),
  };
});

mkdirSync(OUT, { recursive: true });
const path = join(OUT, 'advanced.json');
writeFileSync(path, JSON.stringify({ source: 'docs/design/mockup/Advanced.dc.html', defaults: DEFAULTS, cases }) + '\n');
console.log(`wrote ${path.slice(ROOT.length + 1)} (${cases.length} states)`);
