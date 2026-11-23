#!/usr/bin/env node
// crates/sp-engine/tests/bridge_goldens.mjs — golden data for sp_engine::bridge from the
// Playground mockup's own JavaScript (SPEC ENG-2, ENG-5, BR-5…BR-10).
//
//   node crates/sp-engine/tests/bridge_goldens.mjs   writes crates/sp-engine/tests/bridge_goldens.json
//
// Node 22+, no dependencies. The `class Component` script of
// docs/design/mockup/Playground.dc.html is read verbatim and run on a tiny stand-in for the
// mockup runtime (`DCLogic`: props, state, synchronous setState). Each scenario is a list of
// actions (the same list the Rust test `bridge_goldens.rs` replays on `bridge::Game`); a
// `snap` action records what the mockup's `solve()` and `renderVals()` give at that point.
import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../..');

class DCLogic {
  constructor(props) { this.props = props ?? {}; this.state = {}; }
  setState(patch) { Object.assign(this.state, typeof patch === 'function' ? patch(this.state) : patch); }
}

function loadComponent(screen) {
  const html = readFileSync(join(ROOT, 'docs/design/mockup', `${screen}.dc.html`), 'utf8');
  const m = html.match(/<script type="text\/x-dc" data-dc-script=""[^>]*>([\s\S]*?)<\/script>/);
  if (!m) throw new Error(`no logic script in ${screen}.dc.html`);
  // eslint-disable-next-line no-new-func
  return new Function('DCLogic', `${m[1]}\nreturn Component;`)(DCLogic);
}
const Playground = loadComponent('Playground');

// A pointer event at grid point (x, y) on an unscaled 1120 × 600 stage.
const ev = (x, y) => ({
  currentTarget: { getBoundingClientRect: () => ({ left: 0, top: 0, width: 1120, height: 600 }) },
  clientX: 56 + x * 18, clientY: 380 - y * 18,
});

// ---- scenarios (actions shared with the Rust test)
const howe = (size) => [
  // an 8-joint Howe-style truss over the 40 m gap: bottom 8-18-28-38-48, top 18/28/38 at y = 6
  ['tool', 'member'], ['size', size],
  ['joint', 'J1'], ['grid', 18, 0], ['joint', 'J3'], ['grid', 38, 0], ['joint', 'J2'], ['finish'],
  ['joint', 'J1'], ['grid', 18, 6], ['grid', 28, 6], ['grid', 38, 6], ['joint', 'J2'], ['finish'],
  ['joint', 'J4'], ['joint', 'J6'], ['joint', 'J3'], ['joint', 'J7'], ['finish'],
  ['joint', 'J5'], ['joint', 'J8'], ['joint', 'J3'], ['finish'],
];
const SCENARIOS = {
  'river-empty': [['snap']],
  'river-howe-steel': [
    ...howe('M'), ['snap'],
    ['nums'], ['snap'], ['nums'],
    ['tool', 'select'], ['member', 'M3'], ['snap'],
    ['joint', 'J3'], ['snap'],
    ['joint', 'J6'], ['snap'],
    ['test'], ['tick', 10], ['snap'], ['tick', 30], ['snap'], ['run'], ['snap'],
    ['level', 'sandbox'], ['level', 'river'], ['snap'],
  ],
  'river-howe-thin-timber': [
    ['mat', 'timber'], ...howe('S'), ['snap'],
    ['test'], ['tick', 5], ['snap'], ['run'], ['snap'],
    ['replay'], ['tick', 20], ['snap'], ['run'], ['snap'],
    ['back'], ['snap'],
  ],
  'river-howe-thin-steel': [
    ...howe('S'), ['snap'], ['test'], ['run'], ['snap'], ['tick', 1], ['back'],
    ['tool', 'select'], ['member', 'M8'], ['msize', 'M8', 'L'], ['member', 'M5'], ['msize', 'M5', 'L'],
    ['member', 'M1'], ['snap'], ['test'], ['run'], ['snap'],
  ],
  'river-mechanism': [
    ['tool', 'member'],
    ['joint', 'J1'], ['grid', 18, 0], ['joint', 'J3'], ['grid', 38, 0], ['joint', 'J2'], ['finish'],
    ['joint', 'J1'], ['grid', 8, 6], ['grid', 18, 6], ['grid', 28, 6], ['grid', 38, 6], ['grid', 48, 6], ['joint', 'J2'], ['finish'],
    ['joint', 'J4'], ['joint', 'J7'], ['finish'], ['joint', 'J3'], ['joint', 'J8'], ['finish'],
    ['joint', 'J5'], ['joint', 'J9'], ['finish'],
    ['snap'], ['test'], ['tick', 1], ['snap'], ['run'], ['snap'],
  ],
  'sandbox-loose': [
    ['level', 'sandbox'], ['snap'],
    ['tool', 'joint'], ['grid', 28, 6], ['snap'],
    ['tool', 'weight'], ['weight', 25], ['joint', 'J3'], ['joint', 'J3'], ['snap'],
    ['test'], ['tick', 1], ['snap'], ['run'], ['snap'],
  ],
  // MB-5: the starter truss is cross-braced (see `starterMB5` below), so it holds as dropped in
  'sandbox-starter': [
    ['level', 'sandbox'], ['starter'], ['snap'],
    ['test'], ['run'], ['snap'], ['back'],
    ['tool', 'member'], ['joint', 'J8'], ['joint', 'J4'], ['finish'], ['snap'],
    ['tool', 'weight'], ['weight', 20], ['joint', 'J3'], ['snap'], ['test'], ['run'], ['snap'], ['back'],
    ['undo'], ['weight', 25], ['joint', 'J3'], ['joint', 'J4'], ['weight', 10], ['joint', 'J4'], ['snap'],
    ['mat', 'alu'], ['snap'], ['mat', 'cf'], ['snap'], ['mat', 'timber'], ['snap'],
    ['test'], ['tick', 20], ['snap'], ['run'], ['snap'],
    ['back'], ['mat', 'steel'], ['sag'], ['snap'], ['test'], ['run'], ['snap'],
  ],
  'rules-and-toasts': [
    ['tool', 'joint'], ['grid', 4, -2], ['snap'],
    ['grid', 20, 4], ['snap'],
    ['tool', 'support'], ['joint', 'J3'], ['snap'], ['joint', 'J4'], ['snap'],
    ['tool', 'erase'], ['joint', 'J1'], ['snap'],
    ['tool', 'member'], ['joint', 'J1'], ['joint', 'J4'], ['joint', 'J1'], ['snap'], ['finish'],
    ['joint', 'J4'], ['joint', 'J1'], ['snap'], ['finish'],
    ['test'], ['snap'],
    ['tool', 'weight'], ['joint', 'J3'], ['snap'],
    ['tool', 'select'], ['joint', 'J3'], ['less'], ['snap'], ['less'], ['snap'],
    ['tool', 'joint'], ['grid', 2, 0], ['tool', 'support'], ['joint', 'J5'], ['snap'], ['joint', 'J5'], ['snap'], ['joint', 'J5'], ['snap'],
    ['tool', 'select'], ['down', 'J4'], ['move', 24, 8], ['move', 28, 0], ['move', 26, -2], ['up'], ['snap'],
    ['down', 'J5'], ['move', 4, 2], ['up'], ['snap'],
    ['undo'], ['snap'], ['undo'], ['undo'], ['snap'],
    ['tool', 'erase'], ['member', 'M1'], ['snap'], ['joint', 'J4'], ['snap'],
    ['clear'], ['snap'],
  ],
  'overhang': [
    ['level', 'overhang'], ['snap'],
    ['tool', 'member'], ['joint', 'J2'], ['grid', 14, 2], ['joint', 'J3'], ['grid', 14, 6], ['grid', 8, 6], ['joint', 'J1'], ['finish'],
    ['joint', 'J2'], ['joint', 'J6'], ['finish'], ['joint', 'J4'], ['joint', 'J5'], ['joint', 'J6'], ['finish'], ['joint', 'J4'], ['joint', 'J6'], ['finish'],
    ['joint', 'J5'], ['joint', 'J3'], ['finish'],
    ['snap'], ['test'], ['run'], ['snap'],
  ],
  'haul-piers': [
    ['level', 'haul'],
    ['tool', 'member'],
    ['joint', 'J1'], ['joint', 'J3'], ['grid', 28, 0], ['joint', 'J4'], ['joint', 'J2'], ['finish'],
    ['joint', 'J3'], ['grid', 18, -12], ['finish'], ['joint', 'J4'], ['grid', 38, -12], ['finish'],
    ['joint', 'J1'], ['grid', 18, 6], ['grid', 28, 6], ['grid', 38, 6], ['joint', 'J2'], ['finish'],
    ['joint', 'J3'], ['joint', 'J8'], ['joint', 'J5'], ['joint', 'J9'], ['finish'],
    ['joint', 'J4'], ['joint', 'J10'], ['joint', 'J5'], ['finish'],
    ['tool', 'support'], ['joint', 'J6'], ['joint', 'J7'], ['joint', 'J7'],
    ['snap'], ['test'], ['run'], ['snap'], ['back'],
    ['joint', 'J7'], ['joint', 'J7'], ['snap'], ['test'], ['tick', 30], ['snap'], ['run'], ['snap'],
    ['level', 'river'], ['level', 'haul'], ['snap'],
  ],
};

function snapshot(c) {
  const v = c.renderVals();
  const s = c.state;
  const inSim = (s.phase === 'collapse' || s.phase === 'collapsed') && !!c.sim;
  const r = c.solve(s, inSim ? 1 : (s.phase === 'build' ? 1 : s.lam), s.broken);
  const bands = {};
  ['t1', 't2', 't3', 'c1', 'c2', 'c3', 'z', 'x', 'n'].forEach((b) => ['S', 'M', 'L'].forEach((z) => { if (v[b + z]) bands[b + z] = v[b + z]; }));
  return {
    phase: s.phase, lam: s.lam, level: s.level, mat: s.mat, tool: s.tool,
    nodes: s.nodes.map((n) => [n.id, n.x, n.y, n.sup, n.locked]),
    members: s.members.map((m) => [m.id, m.a, m.b, m.size]),
    loads: Object.entries(s.loads), nN: s.nN, nM: s.nM,
    broken: Object.keys(s.broken),
    log: s.log.map((l) => [l.id ?? null, l.lam, l.mode ?? null, l.bad ?? null, !!l.loose]),
    stars: s.stars ?? 0, best: ['river', 'overhang', 'haul', 'sandbox'].map((k) => s.best[k] || 0),
    toast: s.toast, pending: s.pending, sel: s.sel ? [s.sel.k, s.sel.id] : null, undo: c.hist.length,
    ok: r.ok, bad: r.bad, mass: r.mass, cost: r.cost, worst: r.worst, nAct: r.nAct, ids: r.ids,
    res: Object.entries(r.res).map(([id, q]) => [id, q.N, q.ut, q.mode, q.lim, q.L]),
    u: Object.entries(r.u),
    view: {
      statusLabel: v.statusLabel, statusColor: v.statusColor, costText: v.costText, costColor: v.costColor,
      explain: v.explain, stageLabel: v.stageLabel, surfaceLabel: v.surfaceLabel, hint: v.hint, clock: v.clock,
      testLabel: v.testLabel, showResult: v.showResult, resultTitle: v.showResult ? v.resultTitle : '',
      resultColor: v.showResult ? v.resultColor : '', stars: v.stars, collapsed: v.collapsed, testing: v.testing,
      lamPct: v.lamPct, levelSlug: v.levelSlug, massT: v.massT, nJ: v.nJ, nM: v.nM, canStarter: v.canStarter,
      playing: v.playing, flowCls: v.flowCls, cursor: v.cursor, hasPending: v.hasPending,
      showSizes: v.showSizes, showWeights: v.showWeights, toastOn: v.toastOn,
      joints: v.joints.map((j) => [j.label, j.cls, j.x, j.y]),
      handles: v.memHandles.map((h) => [h.label, h.text, h.cls, h.x, h.y]),
      loadTags: v.loadTags.map((l) => [l.text, l.x, l.y]),
      mi: v.selMember ? { title: v.mi.title, len: v.mi.len, w: String(v.mi.w), ut: v.mi.ut, color: v.mi.color, text: v.mi.text, sizes: v.mi.sizes.map((z) => z.cls) } : null,
      ni: v.selNode ? { title: v.ni.title, pos: v.ni.pos, sups: v.ni.sups.map((z) => z.cls), load: v.ni.load, note: v.ni.note, canDel: v.ni.canDel } : null,
      levels: v.levels.map((l) => [l.name, l.best, l.cls]),
      mats: v.mats.map((m) => m.cls),
      tools: v.tools.map((t) => t.cls),
      bands, backD: v.backD, supD: v.supD, badD: v.badD, sparkD: v.sparkD, ghostD: v.ghostD, hoverD: v.hoverD,
      crateD: v.crateD, cableD: v.cableD,
    },
    sim: c.sim ? { P: c.sim.P.map((p) => [p.x, p.y]), lost: c.sim.lost, sparks: c.sim.sparks.map((k) => [k.x, k.y]), t: c.sim.t, cons: c.sim.cons.length, slow: c.sim.slow } : null,
  };
}

// SPEC MB-5 (lead decision): the mockup's starter truss leaves its middle panel open
// (m + r − 2j = −1, a mechanism). The app cross-braces it with M17 J8–J4 and M18 J9–J3 in
// the same step, so the goldens apply the same two members right after the mockup's own
// `starter()` (one undo step, like the app). Everything else stays the mockup's verbatim.
function starterMB5(c) {
  const s = c.state;
  s.members = s.members.concat([{ id: 'M17', a: 'J8', b: 'J4', size: 'M' }, { id: 'M18', a: 'J9', b: 'J3', size: 'M' }]);
  s.nM = 18;
}

// Undo steps (lead decision (d) + review finding): the app records a step only when the
// design changed — a refused edit (underground spot, pair already joined, joint already at
// 200 t, same size) or a drag that didn't move the joint leaves none. The mockup pushes first
// and checks later, so the harness drops its no-op steps after each action.
const designKey = (s) => JSON.stringify({ nodes: s.nodes, members: s.members, loads: s.loads, nN: s.nN, nM: s.nM });

function run(actions) {
  const c = new Playground({});
  const snaps = [];
  const v = () => c.renderVals();
  for (const a of actions) {
    const [op, p, q] = a;
    const before = designKey(c.state);
    const depth = c.hist.length;
    act(c, v, snaps, op, p, q);
    if (op === 'up') {
      if (c.hist.length && c.hist[c.hist.length - 1] === designKey(c.state)) c.hist.pop();
    } else if (op !== 'down' && op !== 'move' && c.hist.length > depth && designKey(c.state) === before) {
      c.hist.length = depth;
    }
  }
  return snaps;
}

function act(c, v, snaps, op, p, q) {
  {
    switch (op) {
      case 'snap': snaps.push(snapshot(c)); break;
      case 'tool': v().tools[['select', 'member', 'joint', 'support', 'weight', 'erase'].indexOf(p)].pick(); break;
      case 'size': c.setState({ size: p }); break;
      case 'weight': c.setState({ weight: p }); break;
      case 'mat': v().mats[['steel', 'timber', 'alu', 'cf'].indexOf(p)].pick(); break;
      case 'level': v().levels[['river', 'overhang', 'haul', 'sandbox'].indexOf(p)].pick(); break;
      case 'joint': c.clickNode(c.state.nodes.find((n) => n.id === p)); break;
      case 'grid': c.clickSurface(ev(p, q)); break;
      case 'finish': v().finish(); break;
      case 'member': { const h = v().memHandles.find((m) => m.label === 'Member ' + p || m.label.startsWith('Member ' + p + ',')); if (h) h.pick(); break; }
      case 'msize': { const vv = v(); if (vv.selMember) vv.mi.sizes[['S', 'M', 'L'].indexOf(q)].pick(); break; }
      case 'less': { const vv = v(); if (vv.selNode) vv.ni.less(); break; }
      case 'more': { const vv = v(); if (vv.selNode) vv.ni.more(); break; }
      case 'down': { const j = v().joints.find((x) => x.label === 'Joint ' + p || x.label.startsWith('Joint ' + p + ',')); if (j) j.down(); break; }
      case 'move': v().onMove(ev(p, q)); break;
      case 'up': v().onUp(); break;
      case 'undo': v().undo(); break;
      case 'clear': v().clearAll(); break;
      case 'starter': v().starter(); starterMB5(c); break;
      case 'nums': v().toggleNums(); break;
      case 'sag': v().toggleSag(); break;
      case 'test': v().toggleTest(); break;
      case 'back': v().backToBuild(); break;
      case 'replay': v().replay(); break;
      case 'tick': for (let i = 0; i < p; i++) c.tick(); break;
      case 'run': for (let i = 0; i < 2000 && (c.state.phase === 'testing' || c.state.phase === 'collapse'); i++) c.tick(); break;
      default: throw new Error('unknown action ' + op);
    }
  }
}

const out = {};
for (const [name, actions] of Object.entries(SCENARIOS)) out[name] = { actions, snaps: run(actions) };
const file = join(HERE, 'bridge_goldens.json');
writeFileSync(file, JSON.stringify(out, null, 0) + '\n');
console.log(`${file}: ${Object.keys(out).length} scenarios, ${Object.values(out).reduce((n, s) => n + s.snaps.length, 0)} snapshots`);
