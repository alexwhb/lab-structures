#!/usr/bin/env node
// crates/sp-engine/tests/builder_goldens.mjs — golden data for sp_engine::builder from the v2
// builder mockup's own JavaScript (SPEC ENG-5, BLD-*).
//
//   node crates/sp-engine/tests/builder_goldens.mjs            writes builder_goldens.json
//   node crates/sp-engine/tests/builder_goldens.mjs --check    prints it instead (the Rust test
//                                                              compares that with the file)
//
// Node 22+, no dependencies. The `class Component` script of
// docs/design/v2/mockup/Builder.dc.html is read verbatim and run on a tiny stand-in for the
// mockup runtime (`DCLogic`: props, state, synchronous setState) with a fake clock (toasts fade
// only on `wait`). Each scenario starts from a preset (`presetDef()`; `null` = BridgeBuild) and
// is a list of actions — the same list `builder_goldens.rs` replays on `builder::Game`; `snap`
// records the state, the preview / simulation, the result and everything `renderVals()` gives.
//
// Undo steps (lead decision, as v1): the app records a step only when the design changed; the
// mockup pushes first and checks later, so the harness drops its no-op steps after each action.
import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../..');

let NOW = 1_000_000;
Date.now = () => NOW;

class DCLogic {
  constructor(props) { this.props = props ?? {}; this.state = {}; }
  setState(patch) { Object.assign(this.state, typeof patch === 'function' ? patch(this.state) : patch); }
}

const html = readFileSync(join(ROOT, 'docs/design/v2/mockup/Builder.dc.html'), 'utf8');
const m = html.match(/<script type="text\/x-dc" data-dc-script=""[^>]*>([\s\S]*?)<\/script>/);
if (!m) throw new Error('no logic script in Builder.dc.html');
// eslint-disable-next-line no-new-func
const Builder = new Function('DCLogic', `${m[1]}\nreturn Component;`)(DCLogic);

function make(preset) {
  class Board extends Builder { presetDef() { return preset; } }
  return new Board({});
}

// A pointer event at stage px on an unscaled 1120 × 600 stage.
const ev = (px, py) => ({
  currentTarget: { getBoundingClientRect: () => ({ left: 0, top: 0, width: 1120, height: 600 }) },
  clientX: px, clientY: py, deltaY: 0,
});
const atPx = (c, x, y) => { const k = c.cam(); return [k.X(x), k.Y(y)]; };

const BOARDS = {
  BridgeBuild: null,
  BridgeStorm: { level: 'river', build: { kind: 'truss', chord: 'wood' }, cond: { wind: 35, snow: 150, traffic: 'rush' }, run: true, loop: true },
  BridgeCollapse: { level: 'haul', build: { kind: 'truss', chord: 'wood' }, run: true, loop: true },
  BridgeCable: { level: 'sandbox', scene: 'river', build: { kind: 'cable' }, run: true },
  TowerBuild: { level: 't2', build: { kind: 'tower', H: 40, bays: 4, col: 'steel', brace: 'wood', found: 'pile' }, tool: 'select', nums: true },
  TowerStorm: { level: 't2', build: { kind: 'tower', H: 64, bays: 4, col: 'steel', brace: 'wood', found: 'pile' }, run: true, loop: true },
  TowerQuake: { level: 't3', build: { kind: 'tower', H: 32, bays: 2, col: 'steel', brace: 'wood', found: 'footing' }, run: true, loop: true },
};

const roadAcross = [['node', 'J1'], ['at', 16, 0], ['at', 24, 0], ['at', 32, 0], ['at', 40, 0], ['node', 'J2'], ['finish']];

const SCENARIOS = {
  'levels': [
    ['snap'],
    ...['bus', 'haul', 't1', 't2', 't3', 'sandbox'].flatMap((k) => [['level', k], ['snap']]),
    ['scene', 'flat'], ['snap'], ['scene', 'river'], ['snap'], ['level', 'river'], ['snap'],
  ],
  'board-BridgeBuild': { preset: 'BridgeBuild', actions: [['snap'], ['nums'], ['tool', 'select'], ['snap']] },
  'board-TowerBuild': { preset: 'TowerBuild', actions: [['snap'], ['handle', 'M5'], ['snap'], ['node', 'J1'], ['snap'], ['tool', 'build'], ['snap']] },
  'board-BridgeStorm': { preset: 'BridgeStorm', actions: [['snap'], ['run'], ['tick', 20], ['snap'], ['tick', 60], ['snap'], ['toend', 1500], ['snap'], ['tick', 111], ['snap'], ['tick', 5], ['snap']] },
  'board-BridgeCollapse': { preset: 'BridgeCollapse', actions: [['snap'], ['run'], ['tick', 45], ['snap'], ['tick', 90], ['snap'], ['toend', 1500], ['snap'], ['replay'], ['tick', 40], ['snap'], ['back'], ['snap']] },
  'board-BridgeCable': { preset: 'BridgeCable', actions: [['snap'], ['run'], ['tick', 90], ['snap'], ['tick', 400], ['snap'], ['tick', 400], ['snap'], ['back'], ['snap']] },
  'board-TowerStorm': { preset: 'TowerStorm', actions: [['snap'], ['run'], ['tick', 50], ['snap'], ['tick', 150], ['snap'], ['toend', 700], ['snap']] },
  'board-TowerQuake': { preset: 'TowerQuake', actions: [['snap'], ['run'], ['tick', 100], ['snap'], ['tick', 100], ['snap'], ['toend', 700], ['snap']] },
  'river-steel-truss': {
    preset: { level: 'river', build: { kind: 'truss', chord: 'steel' } },
    actions: [['snap'], ['tool', 'select'], ['handle', 'M9'], ['snap'], ['run'], ['tick', 40], ['snap'], ['tick', 100], ['snap'], ['toend', 1500], ['snap'], ['back'], ['snap']],
  },
  'river-wood-truss-rush': {
    preset: { level: 'river', build: { kind: 'truss', chord: 'wood' } },
    actions: [['snap'], ['traffic', 'rush'], ['snap'], ['run'], ['tick', 60], ['snap'], ['toend', 1500], ['snap']],
  },
  'haul-steel-piers': {
    preset: { level: 'haul', build: { kind: 'truss', chord: 'steel' } },
    actions: [['snap'], ['tool', 'anchor'], ['found', 'pile'], ['at', 24, -12], ['at', 32, -12], ['snap'],
      ['tool', 'build'], ['type', 'concrete'], ['node', 'J4'], ['node', 'J11'], ['finish'], ['node', 'J5'], ['node', 'J12'], ['finish'], ['snap'],
      ['run'], ['tick', 60], ['snap'], ['toend', 1500], ['snap']],
  },
  'bus-cable': {
    preset: { level: 'bus' },
    actions: [['snap'], ...roadAcross, ['snap'], ['type', 'steel'], ['node', 'J1'], ['at', 8, 10], ['finish'], ['node', 'J2'], ['at', 48, 10], ['finish'], ['snap'],
      ['type', 'cable'], ['node', 'J9'], ['node', 'J5'], ['node', 'J9'], ['node', 'J6'], ['node', 'J9'], ['node', 'J3'], ['finish'],
      ['node', 'J10'], ['node', 'J8'], ['node', 'J10'], ['node', 'J7'], ['node', 'J10'], ['node', 'J4'], ['finish'], ['snap'],
      ['run'], ['tick', 80], ['snap'], ['toend', 1500], ['snap'], ['back'],
      ['tool', 'select'], ['handle', 'M16'], ['snap'], ['mtype', 'M16', 'rope'], ['snap'], ['run'], ['toend', 1500], ['snap']],
  },
  'edits-and-refusals': [
    ['snap'],
    ...roadAcross, ['snap'],
    ['type', 'steel'], ['node', 'J1'], ['at', 30, 6], ['snap'], ['at', 16, 6], ['snap'], ['node', 'J7'], ['snap'], ['finish'],
    ['type', 'concrete'], ['node', 'J7'], ['node', 'J3'], ['node', 'J4'], ['snap'], ['finish'],
    ['type', 'wood'], ['node', 'J3'], ['node', 'J4'], ['snap'], ['finish'],
    ['tool', 'build'], ['node', 'J7'], ['at', 4, -2], ['snap'], ['finish'],
    ['tool', 'anchor'], ['at', 28, 6], ['snap'], ['at', 28, -12], ['snap'], ['at', 2, 0], ['snap'], ['node', 'J8'], ['snap'], ['node', 'J8'], ['snap'], ['node', 'J1'], ['snap'],
    ['found', 'pile'], ['node', 'J8'], ['snap'],
    ['tool', 'weight'], ['weight', 25], ['node', 'J4'], ['node', 'J4'], ['snap'], ['weight', 10], ['node', 'J5'], ['snap'],
    ['tool', 'select'], ['node', 'J4'], ['snap'], ['less'], ['snap'], ['more'], ['more'], ['more'], ['snap'],
    ['node', 'J7'], ['setfound', 'J7', 'footing'], ['snap'], ['node', 'J2'], ['setfound', 'J2', null], ['snap'],
    ['handle', 'M6'], ['snap'], ['mtype', 'M6', 'rope'], ['snap'], ['mtype', 'M6', 'concrete'], ['snap'], ['mtype', 'M6', 'cable'], ['snap'],
    ['down', 'J7'], ['move', 18, 6], ['move', 18, 8], ['move', 20, -14], ['move', 16, 0], ['up'], ['snap'],
    ['down', 'J8'], ['move', 2, 4], ['up'], ['snap'],
    ['down', 'J4'], ['up'], ['snap'],
    ['undo'], ['snap'], ['undo'], ['undo'], ['snap'],
    ['tool', 'erase'], ['handle', 'M2'], ['snap'], ['node', 'J1'], ['snap'], ['node', 'J5'], ['snap'],
    ['wait', 3], ['snap'],
    ['tool', 'select'], ['node', 'J3'], ['delsel'], ['snap'],
    ['run'], ['snap'], ['tool', 'build'], ['node', 'J4'], ['snap'], ['soil', 'rock'], ['traffic', 'buses'], ['snap'], ['back'],
    ['undo'], ['snap'], ['clear'], ['snap'],
  ],
  'preview-snow-soil': {
    preset: { level: 'river', build: { kind: 'truss', chord: 'wood' } },
    actions: [['snap'], ['snow', 150], ['snap'], ['snow', 400], ['snap'], ['snow', 800], ['snap'], ['nums'], ['snap'],
      ['soil', 'soft'], ['snap'], ['soil', 'loose'], ['snap'], ['soil', 'rock'], ['snap'], ['soil', 'dense'], ['snow', 0], ['snap'],
      ['wind', 25], ['quake', 0.15], ['snap'], ['dir', -1], ['snap'], ['traffic', 'convoy'], ['snap']],
  },
  'mid-run-changes': {
    preset: { level: 't2', build: { kind: 'tower', H: 40, bays: 4, col: 'steel', brace: 'wood', found: 'pile' } },
    actions: [['snap'], ['run'], ['tick', 45], ['snap'], ['wind', 50], ['tick', 30], ['snap'], ['dir', -1], ['snow', 400], ['tick', 30], ['snap'],
      ['quake', 0.5], ['tick', 20], ['snap'], ['shake'], ['tick', 40], ['snap'], ['soil', 'rock'], ['traffic', 'buses'], ['snap'],
      ['slow'], ['tick', 30], ['snap'], ['slow'], ['toend', 700], ['snap']],
  },
  'slow-motion': {
    preset: { level: 'river', build: { kind: 'truss', chord: 'steel' } },
    actions: [['replay'], ['tick', 50], ['snap'], ['slow'], ['tick', 30], ['snap'], ['run'], ['snap']],
  },
  'towers': {
    preset: { level: 't1', build: { kind: 'tower', H: 40, bays: 2, col: 'steel', brace: 'steel', found: 'footing' } },
    actions: [['snap'], ['run'], ['toend', 700], ['snap'], ['back'],
      ['level', 't1'], ['snap'], ['tool', 'build'], ['type', 'concrete'], ['node', 'J1'], ['at', 92, 8], ['at', 108, 8], ['snap'], ['finish'],
      ['node', 'J2'], ['at', 108, 8], ['finish'], ['type', 'floor'], ['node', 'J3'], ['at', 100, 8], ['node', 'J4'], ['finish'], ['type', 'wood'], ['node', 'J1'], ['node', 'J5'], ['finish'], ['snap'], ['run'], ['toend', 700], ['snap']],
  },
  'shaky-ground': {
    preset: { level: 't3', build: { kind: 'tower', H: 32, bays: 2, col: 'steel', brace: 'wood', found: 'pile' } },
    actions: [['snap'], ['run'], ['toend', 700], ['snap'], ['back'], ['quake', 0], ['snap'], ['run'], ['toend', 700], ['snap']],
  },
  'camera': [
    ['snap'], ['zoomin'], ['snap'], ['zoomin'], ['zoomin'], ['zoomin'], ['snap'],
    ['tool', 'build'], ['node', 'J1'], ['at', 9, 1], ['at', 10, 1], ['finish'], ['snap'],
    ['zoomin'], ['zoomin'], ['zoomin'], ['snap'], ['zoomout'], ['zoomout'], ['zoomout'], ['zoomout'], ['zoomout'], ['zoomout'], ['zoomout'], ['zoomout'], ['snap'],
    ['zoomout'], ['zoomout'], ['zoomout'], ['snap'], ['tool', 'build'], ['at', 20, 8], ['finish'], ['snap'],
    ['fit'], ['snap'], ['wheel', -1, 300, 200], ['snap'], ['wheel', 1, 900, 500], ['wheel', 1, 900, 500], ['snap'],
    ['tool', 'select'], ['sdown', 500, 300], ['movepx', 502, 301], ['snap'], ['movepx', 640, 360], ['snap'], ['up'], ['sclick', 640, 360], ['snap'], ['sclick', 640, 360], ['snap'],
    ['tool', 'build'], ['movepx', 400, 250], ['snap'], ['node', 'J1'], ['movepx', 430, 250], ['snap'],
    ['level', 't2'], ['snap'], ['zoomout'], ['zoomout'], ['snap'], ['fit'], ['snap'],
    ['scene', 'flat'], ['level', 'sandbox'], ['scene', 'flat'], ['snap'], ['fit'], ['snap'],
  ],
  'sandbox-flat-build': [
    ['level', 'sandbox'], ['scene', 'flat'], ['snap'],
    ['tool', 'anchor'], ['at', 100, 0], ['snap'], ['at', 100, 8], ['snap'],
    ['tool', 'build'], ['type', 'steel'], ['node', 'J1'], ['at', 92, 12], ['at', 100, 12], ['node', 'J3'], ['finish'], ['node', 'J2'], ['node', 'J5'], ['finish'],
    ['type', 'floor'], ['node', 'J4'], ['node', 'J5'], ['finish'], ['snap'],
    ['type', 'rope'], ['node', 'J5'], ['at', 104, 6], ['finish'], ['tool', 'weight'], ['node', 'J6'], ['snap'],
    ['nums'], ['tool', 'select'], ['snap'], ['run'], ['tick', 120], ['snap'], ['tick', 600], ['snap'], ['back'],
    ['quake', 0.3], ['wind', 35], ['run'], ['tick', 300], ['snap'],
  ],
  'sandbox-river-loops': [
    ['level', 'sandbox'], ...roadAcross, ['type', 'concrete'], ['tool', 'anchor'], ['found', 'pile'], ['at', 24, -12], ['at', 32, -12],
    ['tool', 'build'], ['node', 'J4'], ['node', 'J7'], ['finish'], ['node', 'J5'], ['node', 'J8'], ['finish'], ['snap'],
    ['run'], ['tick', 200], ['snap'], ['tick', 600], ['snap'], ['snow', 800], ['tick', 300], ['snap'],
  ],
};

function chipPick(list, name) { const ch = list.find((x) => x.name === name); if (!ch) throw new Error('no chip ' + name); ch.pick(); }

function act(c, a) {
  const [op, p, q, r] = a;
  const v = () => c.renderVals();
  const T = c.types();
  switch (op) {
    case 'tool': v().tools[['select', 'build', 'anchor', 'weight', 'erase'].indexOf(p)].pick(); break;
    case 'type': chipPick(v().types, T[p].name); break;
    case 'found': c.setState({ found: p }); break;
    case 'weight': chipPick(v().weights, p + ' t'); break;
    case 'node': { const n = c.state.nodes.find((x) => x.id === p); if (n) c.clickNode(n); else if (process.env.DEBUG) console.error('no joint', p, CUR); break; }
    case 'at': { const [px, py] = atPx(c, p, q); c.clickSurface(ev(px, py)); break; }
    case 'finish': v().finish(); break;
    case 'handle': { const h = v().memHandles.find((x) => x.label.split(',')[0].split(' ')[1] === p); if (h) h.pick(); break; }
    case 'mtype': { const vv = v(); if (vv.selMember) chipPick(vv.mi.types, T[q].name); break; }
    case 'setfound': { const vv = v(); if (vv.selNode) vv.ni.founds[[null, 'footing', 'pile'].indexOf(q)].pick(); break; }
    case 'less': { const vv = v(); if (vv.selNode) vv.ni.less(); break; }
    case 'more': { const vv = v(); if (vv.selNode) vv.ni.more(); break; }
    case 'delsel': { const vv = v(); if (vv.selMember) vv.mi.del(); else if (vv.selNode && vv.ni.canDel) vv.ni.del(); break; }
    case 'down': { const j = v().joints.find((x) => x.label.split(',')[0] === 'Joint ' + p); if (j) j.down(); break; }
    case 'move': { const [px, py] = atPx(c, p, q); v().onMove(ev(px, py)); break; }
    case 'movepx': v().onMove(ev(p, q)); break;
    case 'up': v().onUp(); break;
    case 'sdown': v().onSurfaceDown(ev(p, q)); break;
    case 'sclick': v().onSurface(ev(p, q)); break;
    case 'wheel': { const e = ev(q, r); e.deltaY = p; v().onWheel(e); break; }
    case 'zoomin': v().zoomIn(); break;
    case 'zoomout': v().zoomOut(); break;
    case 'fit': v().fitView(); break;
    case 'undo': v().undo(); break;
    case 'clear': v().clearAll(); break;
    case 'level': { const items = v().levelGroups.flatMap((g) => g.items); items[['river', 'bus', 'haul', 't1', 't2', 't3', 'sandbox'].indexOf(p)].pick(); break; }
    case 'scene': v().sbScenes[['river', 'flat'].indexOf(p)].pick(); break;
    case 'wind': v().cWinds[c.winds().findIndex((o) => o[0] === p)].pick(); break;
    case 'dir': v().cDirs[[1, -1].indexOf(p)].pick(); break;
    case 'snow': v().cSnows[c.snows().findIndex((o) => o[0] === p)].pick(); break;
    case 'quake': v().cQuakes[c.quakes().findIndex((o) => o[0] === p)].pick(); break;
    case 'soil': v().cSoils[Object.keys(c.soils()).indexOf(p)].pick(); break;
    case 'traffic': v().cTraffic[c.trafficSets().findIndex((o) => o[0] === p)].pick(); break;
    case 'run': v().toggleRun(); break;
    case 'slow': v().toggleSlow(); break;
    case 'replay': v().replay(); break;
    case 'back': v().backToBuild(); break;
    case 'shake': v().shakeNow(); break;
    case 'nums': v().toggleNums(); break;
    case 'tick': for (let i = 0; i < p; i++) c.tick(); break;
    case 'toend': for (let i = 0; i < p && c.state.phase === 'run'; i++) c.tick(); break;
    case 'wait': NOW += p * 1000; if (c.state.toast && Date.now() - c.state.toastAt > 2600) c.setState({ toast: '' }); break;
    default: throw new Error('unknown action ' + op);
  }
}

const designKey = (s) => JSON.stringify({ nodes: s.nodes, members: s.members, loads: s.loads, nN: s.nN, nM: s.nM });

const PATHS = ['t1', 't2', 't3', 'c1', 'c2', 'c3', 'x', 'woodD', 'steelD', 'concreteD', 'ropeD', 'cableD', 'backD', 'roadTopD', 'roadFrontD', 'roadLineD',
  'facadeD', 'facadeSideD', 'winD', 'snowD', 'foundD', 'supD', 'crateD', 'crateTopD', 'crateSideD', 'crateWireD', 'vCarD', 'vBusD', 'vBoxD', 'vCabD', 'vWinD',
  'vWheelD', 'windArrowD', 'windD', 'flakeD', 'targetD', 'sparkD', 'splashD', 'badD', 'ghostD', 'hoverD', 'soilD', 'bedD', 'waterD', 'flowD', 'topD', 'edgeD', 'grainD'];

function snapshot(c) {
  const s = c.state;
  const v = c.renderVals();
  const live = (s.phase === 'run' || s.phase === 'result') && c.sim;
  const S = live ? c.sim : null;
  const pv = live ? null : c.pv;
  const chips = (l) => l.map((x) => [x.name, x.cls]);
  return {
    state: {
      level: s.level, sbScene: s.sbScene, cond: s.cond, view: [s.view.cx, s.view.cy, s.view.z],
      nodes: s.nodes.map((n) => [n.id, n.x, n.y, n.sup, n.locked]), members: s.members.map((x) => [x.id, x.a, x.b, x.type]),
      loads: Object.entries(s.loads), nN: s.nN, nM: s.nM, pending: s.pending, sel: s.sel ? [s.sel.k, s.sel.id] : null, phase: s.phase,
      tool: s.tool, type: s.type, found: s.found, weight: s.weight, nums: s.nums, toast: s.toast, slow: s.slow, best: s.best, hist: c.hist.length,
      hover: s.hover ? [s.hover.x, s.hover.y] : null, drag: s.drag,
    },
    pv: pv ? {
      res: Object.entries(pv.res).map(([id, r]) => [id, r.N, r.over, r.slack, r.capT, r.capC === Infinity ? null : r.capC, r.L, r.exposed]),
      found: Object.entries(pv.found).map(([id, f]) => [id, f.R, f.Qc, f.Qt, f.overC, f.overT]), moved: pv.moved, movedId: pv.movedId,
    } : null,
    sim: S ? {
      t: S.t, P: S.P.map((p) => [p.x, p.y, p.vx, p.vy, p.ys, p.R, p.sup, p.fstate]), C: S.C.map((k) => [k.id, k.N, k.avg, k.broken, k.slack]),
      V: S.V.map((w) => [w.k, w.x, w.y, w.ang, w.state, w.laps || 0]), firstBreak: S.firstBreak, firstFound: S.firstFound, endAt: S.endAt,
      sway: S.sway, peakA: S.peakA, aS: S.aS, windNow: S.windNow, snowNow: S.snowNow, env: S.env, quakeStart: S.quakeStart,
      sparks: S.sparks.map((k) => [k.x, k.y]), splashes: S.splashes.map((k) => [k.x, k.t]), topI: S.topI, topY0: S.topY0, width: S.width,
    } : null,
    result: s.result,
    view: {
      statusLabel: v.statusLabel, statusColor: v.statusColor, explain: v.explain, stageLabel: v.stageLabel, surfaceLabel: v.surfaceLabel,
      hint: v.hint, clock: v.clock, runInfo: v.runInfo, runLabel: v.runLabel, hdrRun: v.hdrRun, playing: v.playing, running: v.running,
      showResult: v.showResult, resultTitle: v.resultTitle, resultColor: v.resultColor, stars: v.stars, slowLabel: v.slowLabel,
      costText: v.costText, costColor: v.costColor, levelSlug: v.levelSlug, zoomPct: v.zoomPct, goal: v.goal, chipsInfo: v.chipsInfo,
      toastOn: v.toastOn, toast: v.toast, stageCls: v.stageCls, flowCls: v.flowCls, cursor: v.cursor, hasPending: v.hasPending,
      showTypes: v.showTypes, showFounds: v.showFounds, showWeights: v.showWeights, numCls: v.numCls, isSandbox: v.isSandbox, isRiver: v.isRiver,
      canShake: v.canShake, selNone: v.selNone,
      hud: v.hud.map((h) => [h.k, h.v, h.c]),
      joints: v.joints.map((j) => [j.label, j.cls, j.x, j.y]),
      handles: v.memHandles.map((h) => [h.label, h.text, h.cls, h.x, h.y]),
      tags: v.tags.map((l) => [l.text, l.x, l.y]),
      tools: v.tools.map((t) => t.cls), types: chips(v.types), founds: chips(v.founds), weights: chips(v.weights),
      levels: v.levelGroups.map((g) => [g.name, g.items.map((i) => [i.name, i.best, i.cls])]),
      sbScenes: chips(v.sbScenes), cWinds: chips(v.cWinds), cDirs: chips(v.cDirs), cSnows: chips(v.cSnows), cQuakes: chips(v.cQuakes),
      cSoils: chips(v.cSoils), cTraffic: chips(v.cTraffic),
      mi: v.selMember ? { title: v.mi.title, len: v.mi.len, w: v.mi.w, ut: v.mi.ut, color: v.mi.color, text: v.mi.text, types: chips(v.mi.types) } : null,
      ni: v.selNode ? { title: v.ni.title, pos: v.ni.pos, founds: chips(v.ni.founds), load: v.ni.load, note: v.ni.note, canDel: v.ni.canDel } : null,
      typeInfo: v.selNone ? v.typeInfo.map((t) => [t.name, t.curve, t.blurb, t.cost]) : null,
      grid: [v.gridX, v.gridY, v.gridW], soilColor: v.soilColor, bedColor: v.bedColor,
      // a path is written only when it changed since the scenario's last snapshot (null = same)
      paths: Object.fromEntries(PATHS.map((k) => { const same = LAST[k] === v[k]; LAST[k] = v[k]; return [k, same ? null : v[k]]; })),
    },
  };
}

let CUR = '';
let LAST = {};
function run(spec, name) {
  CUR = name;
  LAST = {};
  const { preset, actions } = Array.isArray(spec) ? { preset: null, actions: spec } : spec;
  NOW = 1_000_000;
  const def = typeof preset === 'string' ? BOARDS[preset] : preset;
  const c = make(def);
  const snaps = [];
  for (const a of actions) {
    if (a[0] === 'snap') { snaps.push(snapshot(c)); continue; }
    const before = designKey(c.state);
    const depth = c.hist.length;
    act(c, a);
    if (a[0] === 'up') {
      if (c.hist.length && c.hist[c.hist.length - 1] === designKey(c.state)) c.hist.pop();
    } else if (a[0] !== 'down' && a[0] !== 'move' && a[0] !== 'movepx' && c.hist.length > depth && designKey(c.state) === before) {
      c.hist.length = depth;
    }
  }
  return { preset: def, actions, snaps };
}

// V8's Math.pow(k / 10, 0.2), k = 0…400 (sp_engine::builder::jsmath::POW02_TABLE).
const bits = (x) => { const b = new DataView(new ArrayBuffer(8)); b.setFloat64(0, x); return b.getBigUint64(0).toString(16).padStart(16, '0'); };
const pow02 = []; for (let k = 0; k <= 400; k++) pow02.push(bits(Math.pow(k / 10, 0.2)));

const out = { pow02, scenarios: {} };
for (const [name, spec] of Object.entries(SCENARIOS)) out.scenarios[name] = run(spec, name);
const text = JSON.stringify(out) + '\n';
if (process.argv.includes('--check')) process.stdout.write(text);
else {
  const file = join(HERE, 'builder_goldens.json');
  writeFileSync(file, text);
  const n = Object.values(out.scenarios).reduce((k, s) => k + s.snaps.length, 0);
  console.log(`${file}: ${Object.keys(out.scenarios).length} scenarios, ${n} snapshots, ${(text.length / 1e6).toFixed(1)} MB`);
}
