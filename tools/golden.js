// Renders the reference ("golden") frames from the approved JS drafts, so the
// Rust rig can be proven identical to them. No npm packages.
// usage: node tools/golden.js
'use strict';
const fs = require('fs');
const path = require('path');
const Rig = require('../drafts/looks/rig.js');
require('../drafts/looks/rabbit.js');   // registers SPECIES.rabbit

const OUT = path.join(__dirname, '..', 'tests', 'golden');

// [pet, look, anim, t, toy, collar, lerp]  lerp = null | [anim2, t2, k]
const CASES = [];
const DOG_POSES = [
  ['stand', 0], ['stand', 0.7],
  ['walk', 0], ['walk', 0.225], ['walk', 0.45], ['walk', 0.675],
  ['sit', 0.3],
  ['tilt', 0.2], ['tilt', 1.5], ['tilt', 5.5],
  ['hop', 0.15], ['hop', 0.5], ['hop', 0.85],
  ['sleep', 1], ['sleep', 7.8]
];
for (const look of ['classic', 'detailed']) {
  for (const [anim, t] of DOG_POSES) CASES.push(['dog', look, anim, t, null, false, null]);
  CASES.push(['dog', look, 'walk', 0.1, 'ball', false, null]);
  CASES.push(['dog', look, 'stand', 0.3, 'coconut', false, null]);
  CASES.push(['dog', look, 'stand', 0, null, true, null]);
  CASES.push(['dog', look, 'walk', 0.3, null, false, ['sleep', 1, 0.3333333333333333]]);
}

const TOY_CASES = [['ball', 'classic'], ['coconut', 'classic'], ['football', 'classic']];

function caseName(c) {
  const [pet, look, anim, t, toy, collar, lerp] = c;
  let n = `${pet}-${look}-${anim}-${t}`;
  if (toy) n += `-${toy}`;
  if (collar) n += '-collar';
  if (lerp) n += `-to-${lerp[0]}-${lerp[1]}`;
  return n;
}

function writePpg(name, f) {
  const head = Buffer.alloc(20);
  head.write('PPG1', 0, 'latin1');
  head.writeUInt32LE(f.w, 4); head.writeUInt32LE(f.h, 8);
  head.writeInt32LE(f.ox, 12); head.writeInt32LE(f.gy, 16);
  fs.writeFileSync(path.join(OUT, name + '.ppg'), Buffer.concat([head, Buffer.from(f.data.buffer, f.data.byteOffset, f.data.length)]));
}

fs.mkdirSync(OUT, { recursive: true });
for (const f of fs.readdirSync(OUT)) if (f.endsWith('.ppg')) fs.unlinkSync(path.join(OUT, f));
const lines = [];
for (const c of CASES) {
  const [pet, lookName, anim, t, toy, collar, lerp] = c;
  const look = Rig.LOOKS[lookName];
  let S = Rig.pose(anim, look, t, { species: pet, toy });
  if (lerp) S = Rig.lerpSkel(S, Rig.pose(lerp[0], look, lerp[1], { species: pet, toy }), lerp[2]);
  const name = caseName(c);
  writePpg(name, Rig.render(S, look, { collar }));
  lines.push([name, pet, lookName, anim, String(t), toy || '-', collar ? 1 : 0, lerp ? lerp.map(String).join(':') : '-'].join('|'));
}
for (const [toy, lookName] of TOY_CASES) {
  const name = `toy-${toy}-${lookName}`;
  writePpg(name, Rig.renderToy(toy, Rig.LOOKS[lookName]));
  lines.push([name, 'toy', lookName, '-', '0', toy, 0, '-'].join('|'));
}
fs.writeFileSync(path.join(OUT, 'cases.txt'), lines.join('\n') + '\n');
console.log(`wrote ${lines.length} cases`);
