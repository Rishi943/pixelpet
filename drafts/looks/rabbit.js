/* PixelPet rabbit (draft). Plugs into rig.js as SPECIES.rabbit.
   Default coat: Alex the bun. White "broken" coat with chestnut patches, a
   brown butterfly face with a white blaze, long agouti ears with dark tips.
   Units: head-top height when sitting in a loaf = ~0.9, y up, facing +x. */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory(require('./rig.js'));
  else factory(root.PixelRig);
})(typeof self !== 'undefined' ? self : this, function (Rig) {
  'use strict';
  const { add, sub, mul, len, rot, clamp, mix, TAU, cap, ell, F, H, put, lerpSkel } = Rig.util;

  const BUN = {
    white:  ['#A39A92', '#D3CCC4', '#F3F0EB', '#FFFFFF'],
    brown:  ['#4C3218', '#70502C', '#94693B', '#B48852'],   // agouti chestnut, not orange
    earR:   ['#54402C', '#78603F', '#9A8058', '#B89C74'],
    pink:   ['#9E6F62', '#BC8A7C', '#D6A799', '#E8C2B4'],   // ear insides read pink-tan in the photos
    mask:   ['#20160F', '#2E2118', '#3F2E22', '#584334'],
    black:  ['#0E0B0A', '#171311', '#221C19', '#3A322E'],
    tan:    ['#5A381C', '#86582E', '#AC7B45', '#D0A26C'],
    eye: '#140C09', iris: '#FFFFFF', outline: '#2B1F17', edgeDrop: 2   // white-on-white needs a firmer edge
  };

  // body sliders per look (1 = Alex the bun's real proportions)
  const BODIES = {
    chunky:   { head: 1.38, ear: 0.86, fluff: 1.12, foot: 0.9, limb: 1.6 },
    classic:  { head: 1.14, ear: 0.96, fluff: 1.05, foot: 1, limb: 1.25 },
    detailed: { head: 1, ear: 1, fluff: 1, foot: 1, limb: 1 }
  };
  const bodyFor = look => BODIES[Object.keys(Rig.LOOKS).find(k => Rig.LOOKS[k] === look)] || BODIES.classic;

  // ---------------------------------------------------------------- poses
  // A skeleton is all explicit points; poses are keyframes blended with lerpSkel.
  function loaf(B) {
    return {
      hip: [-0.22, 0.30], sh: [0.12, 0.42], head: [0.30, 0.70 + 0.12 * (B.head - 1)], headRot: -0.05,
      headFront: 0, tilt: 0, earN: 0.10, earF: 0.42, earSw: 0, nose: 0, eyes: 1, breath: 0, flat: 0,
      nf: [[0.17, 0.28], [0.25, 0.035]], ff: [[0.20, 0.29], [0.30, 0.04]],
      nh: [[-0.40, 0.055], [0.05, 0.045]], fh: [[-0.35, 0.06], [0.09, 0.05]],
      tail: [-0.49, 0.30], toy: null
    };
  }
  const k = (o, over) => Object.assign({}, o, over);
  function blend(keys, p) {             // keys: [[phase, skeleton], ...] sorted, p in [0,1)
    for (let i = 0; i < keys.length - 1; i++) {
      const [p0, a] = keys[i], [p1, b] = keys[i + 1];
      if (p >= p0 && p <= p1) { const u = (p - p0) / (p1 - p0 || 1); return lerpSkel(a, b, u * u * (3 - 2 * u)); }
    }
    return keys[keys.length - 1][1];
  }
  function twitch(t) { return (t % 2.6) < 1.1 ? Math.floor(t * 10) % 2 : 0; }
  function blink(t) { return (t % 4.3) > 4.12 ? 0 : 1; }
  function rotAll(S, ang, c) {           // spin the whole bun about c (for a binky)
    const R = q => add(c, rot(sub(q, c), ang));
    const o = Object.assign({}, S);
    for (const key of ['hip', 'sh', 'head', 'tail']) o[key] = R(S[key]);
    for (const key of ['nf', 'ff', 'nh', 'fh']) o[key] = S[key].map(R);
    o.headRot = S.headRot + ang;
    return o;
  }

  const ANIMS = {
    stand: { period: 5.2, fn(B, t) {       // loaf: nose going, ears turning now and then
      const L = loaf(B), br = Math.sin(t * TAU / 2.2);
      const ear = (t % 5.2) > 3.6 && (t % 5.2) < 4.3 ? -0.2 : 0;
      return k(L, { nose: twitch(t), eyes: blink(t), breath: 0.015 * br, earN: L.earN + ear });
    } },
    walk: { period: 0.62, speed: () => 0.72, move: p => (p > 0.06 && p < 0.6 ? 1 / 0.54 : 0), fn(B, t) {
      const L = loaf(B), p = (t / 0.62) % 1;
      const push = k(L, {
        hip: [-0.14, 0.40], sh: [0.26, 0.54], head: [0.47, 0.80 + 0.10 * (B.head - 1)], headRot: -0.12,
        nf: [[0.32, 0.42], [0.50, 0.22]], ff: [[0.34, 0.44], [0.53, 0.25]],
        nh: [[-0.30, 0.30], [-0.62, 0.20]], fh: [[-0.26, 0.31], [-0.57, 0.22]], tail: [-0.40, 0.44], earN: 0.42, earF: 0.5
      });
      const land = k(L, {
        hip: [0.0, 0.44], sh: [0.36, 0.33], head: [0.56, 0.57 + 0.10 * (B.head - 1)], headRot: -0.25,
        nf: [[0.40, 0.22], [0.47, 0.035]], ff: [[0.43, 0.23], [0.52, 0.04]],
        nh: [[-0.12, 0.34], [0.12, 0.22]], fh: [[-0.08, 0.35], [0.16, 0.24]], tail: [-0.24, 0.55], earN: 0.32, earF: 0.4
      });
      return blend([[0, L], [0.22, push], [0.42, land], [0.66, L], [1, L]], p);
    } },
    sit: { period: 1.6, fn(B, t) {          // grooming: sits up and washes the face
      const L = loaf(B), w = Math.sin(t * TAU / 0.8);
      return k(L, {
        hip: [-0.18, 0.30], sh: [0.04, 0.58], head: [0.16, 0.84 + 0.10 * (B.head - 1)], headRot: -0.45,
        nf: [[0.10, 0.50], [0.27, 0.70 + 0.05 * w]], ff: [[0.12, 0.52], [0.29, 0.73 - 0.05 * w]],
        tail: [-0.44, 0.27], eyes: 0, earN: 0.45, earF: 0.55
      });
    } },
    tilt: { period: 4.0, fn(B, t) {         // stands up tall and looks at you
      const L = loaf(B), q = t % 4, sw = q > 1.2 && q < 1.8 ? 0.3 : q > 2.8 && q < 3.3 ? -0.3 : 0;
      return k(L, {
        hip: [-0.12, 0.30], sh: [0.0, 0.76], head: [0.05, 1.12 + 0.12 * (B.head - 1)], headRot: 0,
        headFront: 1, tilt: 0.06 * Math.sin(t * TAU / 4), earSw: sw,
        nf: [[0.08, 0.66], [0.15, 0.53]], ff: [[0.11, 0.67], [0.18, 0.55]],
        nh: [[-0.32, 0.055], [0.10, 0.045]], fh: [[-0.27, 0.06], [0.14, 0.05]],
        tail: [-0.30, 0.24], nose: twitch(t), eyes: blink(t)
      });
    } },
    hop: { period: 1.2, fn(B, t) {          // binky: a happy leap with a twist
      const L = loaf(B), p = (t / 1.2) % 1;
      const crouch = k(L, { hip: [-0.22, 0.25], sh: [0.13, 0.36], head: [0.30, 0.60 + 0.10 * (B.head - 1)], earN: 0.35 });
      let air = k(L, {
        hip: [-0.18, 0.80], sh: [0.18, 0.92], head: [0.38, 1.12 + 0.10 * (B.head - 1)], headRot: 0.1,
        nf: [[0.24, 0.80], [0.38, 0.66]], ff: [[0.27, 0.82], [0.41, 0.69]],
        nh: [[-0.30, 0.70], [-0.58, 0.82]], fh: [[-0.26, 0.72], [-0.52, 0.86]], tail: [-0.44, 0.84], earN: 0.7, earF: 0.85
      });
      air = rotAll(air, 0.32, [0, 0.85]);
      return blend([[0, L], [0.15, crouch], [0.42, air], [0.7, crouch], [0.86, L], [1, L]], p);
    } },
    sleep: { period: 9.6, fn(B, t) {        // the sofa sploot: stretched flat, ears laid back
      const L = loaf(B), br = Math.sin(t * TAU / 3.2);
      const tw = (t % 9.6) > 7.6 && (t % 9.6) < 8.0 ? 0.015 : 0;
      return k(L, {
        hip: [-0.30, 0.16], sh: [0.16, 0.19], head: [0.43, 0.21 + 0.06 * (B.head - 1)], headRot: -0.06, flat: 1,
        nf: [[0.30, 0.10], [0.62, 0.035]], ff: [[0.30, 0.13], [0.58, 0.06]],
        nh: [[-0.48, 0.09], [-0.86, 0.05 + tw]], fh: [[-0.45, 0.12], [-0.80, 0.08]],
        tail: [-0.52, 0.22], earN: 1.5, earF: 1.38, eyes: 0, breath: 0.03 * br
      });
    } }
  };

  // ---------------------------------------------------------------- shapes
  function shapes(S, B, look) {
    const out = [], f = B.fluff, lb = B.limb, br = 1 + S.breath, mr = look.minR / look.R;
    const r = x => Math.max(x, mr);
    const hs = B.head * (S.headFront ? 1.18 : 1.1), hAng = S.headFront ? S.tilt : S.headRot;
    const hp = p => add(S.head, rot(mul(p, hs), hAng));
    const earLen = 0.60 * B.ear, earW = 0.098 * Math.sqrt(B.ear);   // wide and spoon-shaped, as in the photos
    const earShape = (base, ang, part, o) => {
      const dir = rot([0, 1], ang), c = add(base, mul(dir, earLen * 0.5 * hs));
      return ell(c, earLen * 0.5 * hs, earW * hs, Math.atan2(dir[1], dir[0]), part, o);
    };
    const leg = (J, far, grp) => {
      out.push(cap(J[0], J[1], r(0.05 * lb), r(0.042 * lb), 'rleg', { far, grp }));
      out.push(ell(add(J[1], [0.02, 0.0]), r(0.05 * lb), r(0.03 * lb), 0, 'rleg', { far, grp }));
    };
    const foot = (J, far, grp) => out.push(cap(J[0], J[1], r(0.07 * B.foot * lb), r(0.05 * B.foot * lb), 'rfoot', { far, grp }));

    if (!S.headFront) out.push(earShape(hp([-0.11, 0.13]), hAng + S.earF, 'rearF', { far: 1 }));
    foot(S.fh, 1, 3); leg(S.ff, 1, 4);
    // body
    const mid = mul(add(S.hip, S.sh), 0.5), bang = Math.atan2(S.sh[1] - S.hip[1], S.sh[0] - S.hip[0]);
    const flat = S.flat || 0, span = len(sub(S.sh, S.hip));
    out.push(ell(add(mid, [0, 0.03 * (1 - flat)]), (span * 0.5 + 0.22) * f, (0.33 - 0.16 * flat) * f * br, bang * (1 - flat * 0.6), 'rbody'));
    out.push(ell(S.sh, 0.19 * f, (0.25 - 0.10 * flat) * f * br, bang, 'rchest'));
    out.push(ell(S.hip, 0.27 * f, (0.26 - 0.10 * flat) * f, bang * 0.5, 'rhaunch'));
    out.push(ell(S.tail, r(0.085 * f), r(0.08 * f), 0, 'rtail'));
    out.push(ell(mix2(S.sh, S.head, 0.55), 0.15 * f * hs, 0.15 * f * hs, 0, 'rnape'));
    // head
    const headFrom = out.length;
    if (!S.headFront) {
      out.push(ell(hp([0, 0]), 0.21 * hs, 0.18 * hs, hAng, 'rskull'));
      out.push(ell(hp([0.05, -0.07]), 0.17 * hs, 0.135 * hs, hAng, 'rcheek'));
      out.push(ell(hp([0.17, -0.065]), r(0.085 * hs), r(0.078 * hs), hAng, 'rmuzzle'));
      out.push(ell(hp([0.245, -0.03 + 0.012 * S.nose]), r(0.026 * hs), r(0.022 * hs), hAng, 'rnose'));
      out.push(earShape(hp([-0.06, 0.15]), hAng + S.earN, 'rear'));
    } else {
      for (const sg of [-1, 1]) out.push(earShape(hp([0.085 * sg, 0.14]), hAng - 0.2 * sg + (sg < 0 ? S.earSw : 0), 'rfear'));
      out.push(ell(hp([0, 0.01]), 0.20 * hs, 0.185 * hs, hAng, 'rfskull'));
      out.push(ell(hp([0, -0.08]), 0.225 * hs, 0.125 * hs, hAng, 'rfcheek'));
      out.push(ell(hp([0, -0.085]), r(0.075 * hs), r(0.06 * hs), hAng, 'rfmuzzle'));
      out.push(ell(hp([0, -0.055 + 0.01 * S.nose]), r(0.03 * hs), r(0.022 * hs), hAng, 'rnose'));
    }
    for (let i = headFrom; i < out.length; i++) out[i].grp = 6;
    foot(S.nh, 0, 1); leg(S.nf, 0, 2);
    return out;
  }
  const mix2 = (a, b, u) => [mix(a[0], b[0], u), mix(a[1], b[1], u)];

  // ---------------------------------------------------------------- markings
  const near = (x, y, cx, cy, rr) => (x - cx) * (x - cx) + (y - cy) * (y - cy) < rr * rr;
  function paint(s, x, y, look, nz) {
    // Read off the photos: a white face with a brown butterfly over the nose and
    // muzzle, brown rings round the eyes running up to the ear bases, white chin
    // and jowls; a white body with a chain of spots along the back, side flecks
    // and one patch on the rump. Spot edges are ragged like real fur.
    const D = look.detail, fr = nz * (D ? 0.09 * D : 0.04), ux = H.ux, uy = H.uy;
    const spots = list => list.some(([a, b, rr]) => near(ux, uy, a, b, rr + fr));
    switch (s.part) {
      case 'rbody':
        return spots([[-0.66, 0.55, 0.24], [-0.30, 0.86, 0.19], [0.06, 0.93, 0.14], [0.38, 0.84, 0.10],
                      [-0.42, 0.22, 0.12], [0.02, 0.42, 0.08], [-0.70, -0.05, 0.10]]) ? F.brown : F.white;
      case 'rhaunch': return spots([[-0.30, 0.38, 0.44], [0.30, -0.25, 0.14]]) ? F.brown : F.white;
      case 'rchest': case 'rtail': case 'rleg': case 'rfoot': return F.white;
      case 'rnape': return spots([[-0.45, 0.55, 0.22]]) ? F.brown : F.white;
      case 'rskull':                      // profile: eye ring, ear base, bridge down to the muzzle
        if (uy > 0.55 && ux > -0.1 && ux < 0.5) return F.white;              // the white forehead stripe
        if (spots([[0.45, 0.22, 0.38], [-0.25, 0.86, 0.30]])) return F.brown;
        if (ux > 0.55 + fr && uy < 0.5) return F.brown;
        return F.white;
      case 'rcheek': return ux > 0.12 + fr && uy > -0.42 ? F.brown : F.white;   // butterfly wing over the whisker pad
      case 'rmuzzle': return uy < -0.35 + fr ? F.white : F.brown;
      case 'rnose': return F.pink;
      case 'rear':
        if (D >= 1 && Math.abs(uy) < 0.42 && ux > -0.65 && ux < 0.72) return F.pink;
        return ux > 0.84 || (D >= 1 && Math.abs(uy) > 0.86 && ux > 0.55) ? F.mask : F.earR;
      case 'rearF': return ux > 0.84 ? F.mask : F.earR;
      case 'rfear':
        if (Math.abs(uy) < 0.5 && ux > -0.7 && ux < 0.7) return F.pink;
        return ux > 0.8 ? F.mask : F.earR;
      case 'rfskull': {                   // front: eye rings to the ear bases, butterfly bridge, white brow
        const ax = Math.abs(ux);
        if (ax < 0.26 + fr && uy > 0.18) return F.white;                     // white stripe up the forehead
        if (near(ax, uy, 0.66, 0.10, 0.36 + fr) || (ax > 0.3 && uy > 0.55 + fr)) return F.brown;  // eye rings, ear bases
        if (ax < 0.44 + fr && uy <= 0.18) return F.brown;                      // butterfly bridge
        return F.white;
      }
      case 'rfcheek': return Math.abs(ux) < 0.58 + fr && uy > -0.6 + 0.5 * Math.abs(ux) ? F.brown : F.white;    // butterfly wings
      case 'rfmuzzle': return uy < -0.55 ? F.white : F.brown;   // just a white chin
    }
    return F.white;
  }

  function eyes(out, S, B, look, fs, pal) {
    const hs = B.head * (S.headFront ? 1.18 : 1.1), ang = S.headFront ? S.tilt : S.headRot;
    const pts = S.headFront ? [[-0.135, 0.03], [0.135, 0.03]] : [[0.095, 0.04]];
    for (const p of pts) {
      const w = add(S.head, rot(mul(p, hs), ang));
      const ix = Math.floor(fs.ox + w[0] * look.R), iy = Math.floor(fs.gy - w[1] * look.R);
      if (S.eyes > 0.5) {
        put(out, fs, ix, iy, pal.eye);
        if (look.detail >= 1) { put(out, fs, ix + 1, iy, pal.eye); put(out, fs, ix, iy + 1, pal.eye); put(out, fs, ix + 1, iy + 1, pal.eye); put(out, fs, ix + 1, iy, pal.iris); }
      } else {
        put(out, fs, ix, iy + 1, pal.eye); put(out, fs, ix + 1, iy + 1, pal.eye);
        if (look.detail >= 2) put(out, fs, ix - 1, iy + 1, pal.eye);
      }
    }
  }

  Rig.SPECIES.rabbit = { anims: ANIMS, palette: BUN, body: bodyFor, shapes, paint, eyes };
  Rig.BUN = BUN;
  return Rig.SPECIES.rabbit;
});
