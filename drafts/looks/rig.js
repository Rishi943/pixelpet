/* PixelPet rig (draft).
   A pet = a skeleton (joint positions) + body sliders + a coat palette.
   render() rasterises it straight into pixels at any resolution, so one rig
   gives every size and look. Units: withers height = 1, y up, facing +x,
   ground at y = 0. No DOM here: the same file runs in the browser and node. */
(function (root, factory) {
  if (typeof module === 'object' && module.exports) module.exports = factory();
  else root.PixelRig = factory();
})(typeof self !== 'undefined' ? self : this, function () {
  'use strict';

  const add = (a, b) => [a[0] + b[0], a[1] + b[1]];
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1]];
  const mul = (a, s) => [a[0] * s, a[1] * s];
  const len = a => Math.hypot(a[0], a[1]);
  const rot = (a, t) => { const c = Math.cos(t), s = Math.sin(t); return [a[0] * c - a[1] * s, a[0] * s + a[1] * c]; };
  const clamp = (x, a, b) => Math.max(a, Math.min(b, x));
  const mix = (a, b, t) => a + (b - a) * t;
  const TAU = Math.PI * 2;
  const PAW_Y = 0.035;

  // ---------------------------------------------------------------- coat
  // Alex: red-and-black GSD. Sampled from his photos. Each family is
  // [deep, shadow, base, light].
  const ALEX = {
    tan:    ['#6A3518', '#9A5226', '#C47637', '#E09A57'],
    cream:  ['#9C6A3E', '#C68F57', '#E2B077', '#F3D29C'],
    black:  ['#0D0A0A', '#171212', '#231B1B', '#3B3132'],
    mask:   ['#100C0B', '#1C1514', '#2B2220', '#4A3B34'],
    earIn:  ['#3A2218', '#5A3424', '#7E4A34', '#A36B50'],
    nose:   ['#080707', '#100E0E', '#191616', '#3E3838'],
    tongue: ['#8E3442', '#B44E5E', '#D96C7A', '#F09AA4'],
    mouth:  ['#22080B', '#341015', '#46141B', '#5E1E26'],
    collar: ['#132C66', '#1E4494', '#2F64C8', '#5E8FE6'],
    ball:   ['#560910', '#86121E', '#B81F2D', '#E2525C'],
    seam:   ['#B0A088', '#D2C2A6', '#EADBC0', '#FFF4E2'],
    coco:   ['#382010', '#583418', '#7A4C25', '#A26E3E'],
    eye: '#140A06', iris: '#A0601F', outline: '#24120A'
  };
  const FAMS = ['tan', 'cream', 'black', 'mask', 'earIn', 'nose', 'tongue', 'mouth', 'collar', 'ball', 'seam', 'coco',
                'white', 'brown', 'earR', 'pink', 'fwhite', 'fblack'];
  // toys look the same whoever carries them
  const TOYS = {
    ball: ['#560910', '#86121E', '#B81F2D', '#E2525C'], seam: ['#B0A088', '#D2C2A6', '#EADBC0', '#FFF4E2'],
    coco: ['#382010', '#583418', '#7A4C25', '#A26E3E'],
    fwhite: ['#9A9A9E', '#CACACF', '#EEEEF0', '#FFFFFF'], fblack: ['#0C0C0E', '#151518', '#202024', '#3A3A40']
  };
  const F = {}; FAMS.forEach((f, i) => { F[f] = i; });
  const hex = h => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
  function compilePalette(p) {
    return { fam: FAMS.map(f => (p[f] || TOYS[f] || p.tan).map(hex)), eye: hex(p.eye), iris: hex(p.iris), outline: hex(p.outline), edgeDrop: p.edgeDrop || 1 };
  }

  // ---------------------------------------------------------------- looks
  // Body sliders. 1 = Alex's real proportions.
  const BODY = { legLen: 1, bodyLen: 1, head: 1, ear: 1, muzzle: 1, tail: 1, fluff: 1, limb: 1 };
  const LOOKS = {
    chunky:   { label: 'Chunky',   R: 12, zoom: 5, tones: 2, outline: 'hard', minR: 0.9, noise: 0,    dither: 0,    detail: 0,
                body: { legLen: 0.72, bodyLen: 0.84, head: 1.5, ear: 1.15, muzzle: 0.78, tail: 0.9, fluff: 1.12, limb: 1.7 } },
    classic:  { label: 'Classic',  R: 22, zoom: 3, tones: 3, outline: 'hard', minR: 0.8, noise: 0.025, dither: 0,    detail: 1,
                body: { legLen: 0.88, bodyLen: 0.95, head: 1.2, ear: 1.08, muzzle: 0.94, tail: 1, fluff: 1.05, limb: 1.25 } },
    detailed: { label: 'Detailed', R: 36, zoom: 2, tones: 4, outline: 'soft', minR: 0.7, noise: 0.11, dither: 0.08, detail: 2,
                body: { legLen: 1, bodyLen: 1, head: 1, ear: 1, muzzle: 1, tail: 1, fluff: 1, limb: 1 } }
  };
  function frameSize(look) {
    const R = look.R;
    const W = Math.ceil(3.1 * R), H = Math.ceil(2.7 * R);
    return { W, H, ox: Math.round(1.6 * R), gy: H - 2 };
  }

  // ---------------------------------------------------------------- skeleton
  function ik(A, C, l1, l2, bend) {
    let d = sub(C, A), dist = len(d);
    dist = clamp(dist, Math.abs(l1 - l2) + 1e-4, l1 + l2 - 1e-4);
    const a = Math.acos(clamp((l1 * l1 + dist * dist - l2 * l2) / (2 * l1 * dist), -1, 1));
    const ang = Math.atan2(d[1], d[0]) + bend * a;
    return [A[0] + l1 * Math.cos(ang), A[1] + l1 * Math.sin(ang)];
  }

  function geom(B) {
    const lk = B.legLen, bl = B.bodyLen, by = 0.24 + 0.46 * lk;
    const hip = [-0.38 * bl, by], sh = [0.30 * bl, by + 0.02];
    const fTop = add(sh, [0, -0.10]), fPaw = [sh[0] + 0.03, PAW_Y];
    const fW = add(fPaw, [-0.02, 0.10 * lk]);
    const hTop = add(hip, [0.03, -0.06]), hPaw = [hip[0] - 0.10, PAW_Y];
    const hH = add(hPaw, [-0.05, 0.17 * lk]);
    const fd = len(sub(fW, fTop)), hd = len(sub(hH, hTop));
    return { by, hip, sh, fPaw, hPaw, f1: fd * 0.53, f2: fd * 0.53, h1: hd * 0.6, h2: hd * 0.6,
             restAng: Math.atan2(sh[1] - hip[1], sh[0] - hip[0]) };
  }

  const HANG = [-2.45, -1.95, -1.72, -2.05];

  function tailChain(base, angs, seg) {
    const pts = [base]; let p = base;
    for (const a of angs) { p = add(p, [Math.cos(a) * seg, Math.sin(a) * seg]); pts.push(p); }
    return pts;
  }

  function frontLeg(top, paw, flex, B, G) {
    const wrist = add(paw, [-0.02 + 0.09 * flex, (0.10 - 0.035 * flex) * B.legLen]);
    return [top, ik(top, wrist, G.f1, G.f2, -1), wrist, paw];
  }
  function hindLeg(top, paw, flex, B, G) {
    const hock = add(paw, [-0.05 - 0.06 * flex, (0.17 - 0.05 * flex) * B.legLen]);
    return [top, ik(top, hock, G.h1, G.h2, 1), hock, paw];
  }

  // Fill in everything derived (legs by IK, tail, neck) from a few drivers.
  function build(B, G, c) {
    const ang = Math.atan2(c.sh[1] - c.hip[1], c.sh[0] - c.hip[0]) - G.restAng;
    const S = {
      hip: c.hip, sh: c.sh, head: c.head, headRot: c.headRot || 0,
      headFront: c.headFront || 0, tilt: c.tilt || 0,
      mouth: c.mouth || 0, tongue: c.tongue || 0, eyes: c.eyes == null ? 1 : c.eyes,
      earBack: c.earBack || 0, breath: c.breath || 0, toy: c.toy || null, legs: {}
    };
    const fl = c.flex || {};
    const fTop = add(c.sh, rot([0, -0.10], ang)), hTop = add(c.hip, rot([0.03, -0.06], ang));
    const L = c.legs || {};
    S.legs.nf = L.nf || frontLeg(fTop, c.paws.nf, fl.nf || 0, B, G);
    S.legs.ff = L.ff || frontLeg(add(fTop, [0.03, 0.01]), c.paws.ff, fl.ff || 0, B, G);
    S.legs.nh = L.nh || hindLeg(hTop, c.paws.nh, fl.nh || 0, B, G);
    S.legs.fh = L.fh || hindLeg(add(hTop, [0.03, 0.01]), c.paws.fh, fl.fh || 0, B, G);
    const tb = add(c.hip, rot(c.tailBase || [-0.20, 0.06], ang));
    S.tail = tailChain(tb, c.tailAngs || HANG, 0.19 * B.tail * (c.tailLen || 1));
    S.neckA = add(c.sh, rot([0.07, 0.14], ang));
    S.neckB = add(c.head, rot([-0.07 * B.head, -0.07 * B.head], S.headFront ? 0 : S.headRot));
    return S;
  }

  function restPaws(G) {
    return { nf: G.fPaw, ff: add(G.fPaw, [0.06, 0]), nh: G.hPaw, fh: add(G.hPaw, [0.06, 0]) };
  }
  function headRest(B, G) { return add(G.sh, [0.29, 0.52 + 0.07 * (B.head - 1)]); }

  // ---------------------------------------------------------------- animations
  // Each returns a skeleton for time t (seconds). Periods are for looping.
  const ANIMS = {
    stand: { period: 2.0, fn(B, t, o) {
      const G = geom(B), pant = Math.sin(t * TAU * 2.5);
      return build(B, G, {
        hip: G.hip, sh: add(G.sh, [0, 0.004 * pant]), head: add(headRest(B, G), [0, 0.006 * pant]), headRot: 0.08,
        paws: restPaws(G), mouth: o.toy ? 0.45 : 0.42 + 0.12 * pant, tongue: o.toy ? 0 : 1, breath: 0.012 * pant,
        tailAngs: HANG.map((a, i) => a + Math.sin(t * TAU * 1.5) * 0.10 * (i + 1) * 0.5), toy: o.toy
      });
    } },
    walk: { period: 0.9, fn(B, t, o) {
      const G = geom(B), p = (t / 0.9) % 1;
      const stride = 0.30 * B.legLen, duty = 0.62, lift = 0.10 * B.legLen;
      const offs = { nh: 0, nf: 0.25, fh: 0.5, ff: 0.75 }, rest = restPaws(G), paws = {}, flex = {};
      for (const k in offs) {
        const ph = (p + offs[k]) % 1; let dx, dy = 0, f = 0;
        if (ph < duty) dx = stride * (0.5 - ph / duty);
        else { const s = (ph - duty) / (1 - duty); dx = stride * (-0.5 + s); dy = lift * Math.sin(Math.PI * s); f = Math.sin(Math.PI * s); }
        paws[k] = add(rest[k], [dx, dy]); flex[k] = f;
      }
      const bob = 0.014 * Math.cos(p * TAU * 2);
      return build(B, G, {
        hip: add(G.hip, [0, bob]), sh: add(G.sh, [0, bob * 0.7]),
        head: add(headRest(B, G), [0.01, -0.02 + 0.012 * Math.cos(p * TAU * 2 + 0.8)]), headRot: 0.0,
        paws, flex, mouth: o.toy ? 0.45 : 0.35, tongue: o.toy ? 0 : 0.7,
        tailAngs: HANG.map((a, i) => a + 0.05 + Math.sin(p * TAU) * 0.08 * (i + 1) * 0.5), toy: o.toy
      });
    } },
    sit: { period: 2.0, fn(B, t, o) { return sitSkel(B, t, o, false); } },
    tilt: { period: 4.0, fn(B, t, o) { return sitSkel(B, t, o, true); } },
    hop: { period: 1.0, fn(B, t, o) {
      const G = geom(B), p = (t / 1.0) % 1;
      let lift = 0, crouch = 0, air = 0, pitch = 0;
      if (p < 0.22) crouch = 0.09 * Math.sin(Math.PI / 2 * p / 0.22);
      else if (p < 0.78) { const s = (p - 0.22) / 0.56; lift = 0.5 * B.legLen * Math.sin(Math.PI * s); air = Math.sin(Math.PI * s); pitch = 0.06 * Math.cos(Math.PI * s); }
      else crouch = 0.09 * Math.cos(Math.PI / 2 * (p - 0.78) / 0.22);
      const dy = lift - crouch, rest = restPaws(G);
      const paws = air > 0 ? {
        nf: add(rest.nf, [0.10 * air, lift + 0.10 * air]), ff: add(rest.ff, [0.12 * air, lift + 0.12 * air]),
        nh: add(rest.nh, [-0.10 * air, lift + 0.04 * air]), fh: add(rest.fh, [-0.08 * air, lift + 0.05 * air])
      } : rest;
      return build(B, G, {
        hip: add(G.hip, [0, dy - crouch * 0.4 - pitch]), sh: add(G.sh, [0, dy + pitch]),
        head: add(headRest(B, G), [0.02 * air, dy + pitch * 1.4]), headRot: -0.12 + pitch,
        paws, flex: { nf: 0.9 * air, ff: 0.9 * air, nh: 0.5 * air, fh: 0.5 * air },
        mouth: o.toy ? 0.45 : 0.6, tongue: o.toy ? 0 : 0.6, earBack: 0.35 * air,
        tailAngs: HANG.map((a, i) => a + air * 0.5 * (i + 1) * 0.35), toy: o.toy
      });
    } },
    sleep: { period: 9.6, fn(B, t, o) {
      const G = geom(B), bl = B.bodyLen, f = B.fluff, br = Math.sin(t * TAU / 3.2);
      // lying flat on his side, legs stretched forward (from Alex's sleeping photo)
      const hip = [-0.40 * bl, 0.27 * f], sh = [0.18 * bl, 0.31 * f];
      const tw = (t % 9.6) > 7.4 && (t % 9.6) < 8.2 ? Math.sin((t % 9.6) * 30) * 0.018 : 0;  // dream twitch
      const fx = sh[0], hx = hip[0];
      const nf = [[fx + 0.10, 0.16], [fx + 0.28, 0.07], [fx + 0.62, 0.055 + tw], [fx + 0.74, 0.05 + tw]];
      const ff = nf.map(q => add(q, [-0.02, 0.07]));
      const nh = [[hx + 0.12, 0.18], [hx + 0.36, 0.10], [hx + 0.24, 0.05], [hx + 0.44, 0.05 + tw * 0.5]];
      const fh = nh.map(q => add(q, [0.06, 0.05]));
      return build(B, G, {
        hip, sh, head: [fx + 0.36, 0.29 + 0.08 * (B.head - 1)], headRot: -0.12, eyes: 0, earBack: 0.12,
        breath: 0.035 * br, legs: { nf, ff, nh, fh }, paws: restPaws(G),
        tailBase: [-0.20, -0.04], tailAngs: [-2.2, -2.75, -3.1, 2.75], tailLen: 0.9
      });
    } }
  };

  function sitSkel(B, t, o, tilt) {
    // measured off Alex's side-on sitting photo, in withers units, from the front paws
    const G = geom(B), lk = B.legLen, bl = B.bodyLen, f = B.fluff;
    const P = 0.20 * bl;
    const sh = [P - 0.15 * bl, 0.75 * lk], hip = [P - 0.52 * bl, 0.26 * f];
    const pant = Math.sin(t * TAU * 2.5);
    const hTop = add(hip, [0.06, 0.02]);
    const nh = [hTop, [hip[0] + 0.28, 0.20], [hip[0] - 0.02, 0.07], [hip[0] + 0.26, PAW_Y]];
    const fh = nh.map(q => add(q, [0.05, 0.01]));
    let tiltA = 0, mouth = o.toy ? 0.45 : 0.42 + 0.12 * pant, tongue = o.toy ? 0 : 1;
    if (tilt) {
      const k = t % 4.0;
      const ramp = k < 0.5 ? 0 : k < 0.9 ? (k - 0.5) / 0.4 : k < 2.2 ? 1 : k < 2.6 ? 1 - (k - 2.2) / 0.4 : 0;
      tiltA = 0.34 * ramp * (Math.floor(t / 4.0) % 2 ? -1 : 1);
      mouth = o.toy ? 0.45 : 0; tongue = 0;
    }
    return build(B, G, {
      hip, sh, head: add(sh, [tilt ? 0.17 : 0.20, (tilt ? 0.50 : 0.57) * lk + 0.06 * (B.head - 1) + (tilt ? 0 : 0.004 * pant)]), headRot: 0.22,
      headFront: tilt ? 1 : 0, tilt: tiltA, paws: { nf: [P, PAW_Y], ff: [P + 0.06, PAW_Y] },
      legs: { nh, fh }, mouth: mouth * 0.75, tongue, breath: 0.01 * pant,
      tailBase: [-0.20, -0.06], tailAngs: [-2.45 + 0.1 * Math.sin(t * TAU * 1.5), -2.95, 2.55, 1.95], tailLen: 0.8, toy: o.toy
    });
  }

  const DISCRETE = { headFront: 1, eyes: 1, toy: 1 };
  function lerpSkel(a, b, k) {
    if (Array.isArray(a)) return a.map((x, i) => lerpSkel(x, b[i], k));
    if (typeof a === 'number' && typeof b === 'number') return mix(a, b, k);
    if (a && typeof a === 'object') {
      const o = {};
      for (const key in b) o[key] = !(key in a) ? b[key] : DISCRETE[key] ? (k < 0.5 ? a[key] : b[key]) : lerpSkel(a[key], b[key], k);
      return o;
    }
    return k < 0.5 ? a : b;
  }

  function walkSpeed(B) { return (0.30 * B.legLen) / (0.62 * 0.9); }  // units per second, no foot slide

  // ---------------------------------------------------------------- shapes
  const cap = (a, b, ra, rb, part, o) => Object.assign({ k: 'cap', a, b, ra, rb, part, grp: 0 }, o);
  const ell = (c, rx, ry, ang, part, o) => Object.assign({ k: 'ell', c, rx, ry, ang, part, grp: 0 }, o);
  const tri = (p0, p1, p2, part, o) => Object.assign({ k: 'tri', p: [p0, p1, p2], part, grp: 0 }, o);

  function shapes(S, B, look) {
    const out = [], f = B.fluff, lb = B.limb, br = 1 + S.breath, mr = look.minR / look.R;
    const r = x => Math.max(x, mr);
    const hs = B.head * (S.headFront ? 1.15 : 1);
    const hAng = S.headFront ? S.tilt : S.headRot;
    const hp = p => add(S.head, rot(mul(p, hs), hAng));
    const torsoAng = Math.atan2(S.sh[1] - S.hip[1], S.sh[0] - S.hip[0]);

    // ears behind the head
    const earTip = (base0, base1, lenE, lean) => {
      const m = mul(add(base0, base1), 0.5);
      return add(m, rot([0, lenE], lean));
    };
    if (!S.headFront) {
      const b0 = [-0.18, 0.08], b1 = [-0.05, 0.11];
      out.push(tri(hp(b0), hp(b1), hp(earTip(b0, b1, 0.25 * B.ear, 0.30 + 1.2 * S.earBack)), 'earF', { far: 1 }));
    }
    // far legs
    legShapes(out, S.legs.fh, 'h', 1, 3);
    legShapes(out, S.legs.ff, 'f', 1, 4);
    // tail
    const tr = [0.06, 0.085, 0.09, 0.075, 0.035];
    for (let i = 0; i < 4; i++) out.push(cap(S.tail[i], S.tail[i + 1], r(tr[i] * f), r(tr[i + 1] * f), 'tail', { idx: i, flip: 1, grp: 5 }));
    // body
    out.push(ell(S.hip, 0.23 * f, 0.23 * f * br, torsoAng, 'rump'));
    out.push(cap(S.hip, S.sh, 0.21 * f * br, 0.26 * f * br, 'torso'));
    out.push(ell(add(S.sh, rot([0.04, -0.03], torsoAng)), 0.25 * f, 0.28 * f * br, torsoAng, 'chest'));
    out.push(cap(S.neckA, S.neckB, 0.19 * f, 0.14 * f * B.head, 'neck'));
    out.push(ell(add(S.sh, [0.10, -0.08]), 0.17 * f, 0.20 * f * br, 0, 'brisket'));
    // head
    const headFrom = out.length;
    if (!S.headFront) {
      const m = S.mouth, mz = B.muzzle;
      const jawA = [0.06, -0.095], jawB = add(jawA, rot([0.20 * mz, 0], -0.08 - 0.32 * m));
      const upTip = [0.09 + 0.22 * mz, -0.075];
      if (m > 0.08) out.push(tri(hp([0.05, -0.08]), hp(add(upTip, [0, -0.03])), hp(jawB), 'mouth'));
      out.push(cap(hp(jawA), hp(jawB), 0.06 * hs, r(0.04 * hs), 'jaw'));
      out.push(ell(hp([0, 0]), 0.19 * hs, 0.155 * hs, hAng, 'skull'));
      if (S.tongue > 0.05) {
        const tA = [0.11 + 0.06 * mz, -0.11], tB = add(tA, [0.05, -0.15 * S.tongue - 0.02]);
        out.push(cap(hp(tA), hp(tB), r(0.032 * hs), r(0.04 * hs), 'tongue'));
      }
      out.push(cap(hp([0.09, -0.035]), hp(upTip), 0.095 * hs, 0.058 * hs, 'muzzle'));
      out.push(ell(hp(add(upTip, [0.012, 0.008])), r(0.042 * hs), r(0.032 * hs), hAng, 'nose'));
      const b0 = [-0.11, 0.09], b1 = [0.04, 0.12];
      out.push(tri(hp(b0), hp(b1), hp(earTip(b0, b1, 0.27 * B.ear, 0.24 + 1.2 * S.earBack)), 'earN'));
    } else {
      for (const sgn of [-1, 1]) {
        const b0 = [0.18 * sgn, 0.05], b1 = [0.05 * sgn, 0.13];
        out.push(tri(hp(b0), hp(b1), hp([0.19 * sgn, 0.08 + 0.29 * B.ear]), 'earFront', { sgn }));
      }
      out.push(ell(hp([0, 0.02]), 0.19 * hs, 0.16 * hs, hAng, 'fskull'));
      out.push(ell(hp([0, -0.07]), 0.16 * hs, 0.095 * hs, hAng, 'fcheek'));
      if (S.mouth > 0.08) out.push(ell(hp([0, -0.16]), 0.055 * hs, 0.03 * hs * S.mouth + 0.01, hAng, 'mouth'));
      out.push(ell(hp([0, -0.10]), 0.07 * hs, 0.065 * hs, hAng, 'fmuzzle'));
      out.push(cap(hp([0, -0.01]), hp([0, -0.09]), r(0.035 * hs), r(0.045 * hs), 'fmuzzle'));
      if (S.tongue > 0.05) out.push(cap(hp([0, -0.155]), hp([0, -0.165 - 0.1 * S.tongue]), r(0.035 * hs), r(0.04 * hs), 'tongue'));
      out.push(ell(hp([0, -0.075]), r(0.045 * hs), r(0.03 * hs), hAng, 'nose'));
    }
    for (let i = headFrom; i < out.length; i++) out[i].grp = 6;
    // near legs
    legShapes(out, S.legs.nh, 'h', 0, 1);
    legShapes(out, S.legs.nf, 'f', 0, 2);
    // toy in mouth
    if (S.toy) {
      const big = S.toy === 'coconut';
      const at = S.headFront ? hp([0, big ? -0.22 : -0.19]) : hp([0.20 * B.muzzle + (big ? 0.06 : 0.07), big ? -0.15 : -0.12]);
      out.push(toyShape(S.toy, at));
    }
    return out;

    function legShapes(o, J, kind, far, grp) {
      const [a, b, c, d] = J;
      if (kind === 'f') {
        o.push(cap(a, b, r(0.085 * lb * f), r(0.06 * lb), 'legU', { far, grp, noEdge: 1 }));
        o.push(cap(b, c, r(0.05 * lb), r(0.042 * lb), 'leg', { far, grp }));
        o.push(cap(c, d, r(0.04 * lb), r(0.038 * lb), 'leg', { far, grp }));
      } else {
        o.push(cap(a, b, r(0.15 * f * Math.sqrt(lb)), r(0.075 * lb), 'thigh', { far, grp, noEdge: 1 }));
        o.push(cap(b, c, r(0.062 * lb), r(0.042 * lb), 'leg', { far, grp }));
        o.push(cap(c, d, r(0.04 * lb), r(0.036 * lb), 'leg', { far, grp }));
      }
      o.push(ell(add(d, [0.028, 0.004]), r(0.055 * lb), r(0.03 * lb), 0, 'paw', { far, grp }));
    }
  }

  function toyShape(type, at) {
    if (type === 'football') return ell(at, 0.085, 0.085, 0.2, 'fball', { grp: 9 });
    return type === 'coconut' ? ell(at, 0.115, 0.10, 0.3, 'coco', { grp: 9 }) : ell(at, 0.072, 0.072, 0.5, 'ball', { grp: 9 });
  }

  // ---------------------------------------------------------------- raster
  const LIGHT = (() => { const v = [0.35, 0.75, 0.56], n = Math.hypot(...v); return v.map(x => x / n); })();
  function litFrom(nx, ny) {
    const nz = Math.sqrt(Math.max(0, 1 - nx * nx - ny * ny));
    return nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2];
  }
  function hash(x, y) {
    let h = (Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263)) | 0;
    h = Math.imul(h ^ (h >>> 13), 1274126177);
    return ((h ^ (h >>> 16)) & 1023) / 511.5 - 1;
  }
  function segDist(p, a, b) {
    const dx = b[0] - a[0], dy = b[1] - a[1];
    const t = clamp(((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy || 1e-9), 0, 1);
    return Math.hypot(p[0] - a[0] - dx * t, p[1] - a[1] - dy * t);
  }

  const H = { lit: 0, t: 0, v: 0, ux: 0, uy: 0, e0: 0, e1: 0, h: 0 };
  function hit(s, x, y) {
    if (s.k === 'cap') {
      const dx = s.b[0] - s.a[0], dy = s.b[1] - s.a[1], L2 = dx * dx + dy * dy || 1e-9;
      const t = clamp(((x - s.a[0]) * dx + (y - s.a[1]) * dy) / L2, 0, 1);
      const r = mix(s.ra, s.rb, t), ex = x - s.a[0] - dx * t, ey = y - s.a[1] - dy * t;
      if (ex * ex + ey * ey > r * r) return false;
      const ln = Math.sqrt(L2);
      H.t = t; H.v = (ex * -dy / ln + ey * dx / ln) / r * (s.flip ? -1 : 1);
      H.lit = litFrom(ex / r, ey / r);
      return true;
    }
    if (s.k === 'ell') {
      const c = Math.cos(s.ang), sn = Math.sin(s.ang), lx = x - s.c[0], ly = y - s.c[1];
      const ux = (lx * c + ly * sn) / s.rx, uy = (-lx * sn + ly * c) / s.ry;
      if (ux * ux + uy * uy > 1) return false;
      H.ux = ux; H.uy = uy; H.v = uy; H.t = 0;
      H.lit = litFrom(ux * c - uy * sn, ux * sn + uy * c);
      return true;
    }
    const [a, b, c] = s.p;
    const d1 = (x - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (y - b[1]);
    const d2 = (x - c[0]) * (b[1] - c[1]) - (b[0] - c[0]) * (y - c[1]);
    const d3 = (x - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (y - a[1]);
    if (((d1 < 0) || (d2 < 0) || (d3 < 0)) && ((d1 > 0) || (d2 > 0) || (d3 > 0))) return false;
    const P = [x, y];
    H.e0 = segDist(P, a, c); H.e1 = segDist(P, b, c);
    const m = mul(add(a, b), 0.5), hl = len(sub(c, m));
    H.h = clamp(((x - m[0]) * (c[0] - m[0]) + (y - m[1]) * (c[1] - m[1])) / (hl * hl), 0, 1);
    H.lit = 0.5 + 0.25 * H.h;
    return true;
  }

  // Coat markings: which colour family a point on a body part gets.
  function paint(s, x, y, look, nz) {
    const D = look.detail, fr = D ? nz * 0.10 * D : 0;
    switch (s.part) {
      case 'torso': {
        const t = H.t, thr = -0.22 + 1.5 * Math.max(0, 0.2 - t) + 1.8 * Math.max(0, t - 0.7);
        if (H.v > thr + fr) return F.black;
        return H.v < -0.62 ? F.cream : F.tan;
      }
      case 'rump':
        if (H.uy > -0.02 + fr && H.ux > -0.45 + 0.3 * H.uy) return F.black;
        return H.ux < -0.45 && H.uy < 0.1 ? F.cream : F.tan;
      case 'brisket': return H.uy < -0.55 ? F.cream : F.tan;
      case 'chest':
        if (H.uy > 0.5 + fr && H.ux < 0.0) return F.black;
        return H.uy < -0.6 ? F.cream : F.tan;
      case 'neck':
        if (s.collar && H.t > 0.5 && H.t < 0.68) return F.collar;
        if (H.v > 0.45 + fr * 2 && H.t < 0.4) return F.black;
        return F.tan;
      case 'skull': {
        const ex = H.ux - 0.5, ey = H.uy - 0.26;
        if (D >= 1 && Math.hypot(H.ux - 0.40, H.uy - 0.62) < 0.15) return F.cream;     // eyebrow spot
        if (D >= 2 && ex > -0.2 && ex < 0.55 && Math.abs(ey + 0.35 * ex) < 0.1) return F.mask;  // eye line to muzzle
        if (H.ux > 0.55 + fr && H.uy < 0.15) return F.mask;
        if (D >= 1 && H.uy > 0.72 + fr && H.ux < -0.1) return F.black;                // dark crown
        return F.tan;
      }
      case 'muzzle': return H.t < 0.32 && H.v < -0.15 ? F.tan : F.mask;
      case 'jaw': return H.t > 0.42 ? F.mask : F.tan;
      case 'nose': return F.nose;
      case 'tongue': return F.tongue;
      case 'mouth': return F.mouth;
      case 'earN':
        if (D === 0) return F.black;
        return (H.e0 < 0.035 || H.e1 < 0.022 || H.h > 0.8) ? F.black : F.earIn;
      case 'earF': return F.black;
      case 'earFront':
        if (D === 0) return F.black;
        return (H.e0 < 0.03 || H.e1 < 0.03 || H.h > 0.8) ? F.black : F.earIn;
      case 'fskull': {
        if (D >= 1 && Math.abs(H.ux) < 0.07 && H.uy > 0.2) return F.mask;              // forehead line
        if (D >= 1 && Math.hypot(Math.abs(H.ux) - 0.40, H.uy - 0.48) < 0.13) return F.cream;  // eyebrows
        if (D >= 2 && Math.abs(H.ux) < 0.62 && H.uy < 0.12 - 0.4 * Math.abs(H.ux) + fr) return F.mask;
        return F.tan;
      }
      case 'fcheek': return Math.abs(H.ux) < 0.36 + fr ? F.mask : F.tan;
      case 'fmuzzle': return F.mask;
      case 'tail':
        if (s.idx === 3 && H.t > 0.35) return F.black;
        if (H.v > -0.2 + fr) return F.black;
        return H.v < -0.6 ? F.cream : F.tan;
      case 'thigh':
        if (H.t < 0.32 && H.v > 0.25 + fr) return F.black;
        return H.v < -0.35 ? F.cream : F.tan;
      case 'legU': return H.v < -0.4 ? F.cream : F.tan;
      case 'leg': return H.v < 0.15 ? F.cream : F.tan;
      case 'paw': return F.cream;
      case 'ball': return Math.abs(H.ux - 0.15) < 0.13 ? F.seam : F.ball;
      case 'coco': return F.coco;
      case 'fball': return [[0, 0.05], [0.8, 0.35], [-0.8, 0.35], [0.5, -0.75], [-0.5, -0.75]].some(([a, b]) => Math.hypot(H.ux - a, H.uy - b) < 0.3) ? F.fblack : F.fwhite;
    }
    return F.tan;
  }

  const BAYER = [0, 2, 3, 1];
  function toneBand(l, look) {
    if (look.tones === 2) return l < 0.32 ? 1 : 2;
    if (look.tones === 3) return l < 0.30 ? 1 : l > 0.84 ? 3 : 2;
    return l < 0.10 ? 0 : l < 0.36 ? 1 : l > 0.80 ? 3 : 2;
  }
  function toneOf(lit, look, nz, ix, iy) {
    const t0 = toneBand(lit, look);
    const t1 = toneBand(lit + look.noise * nz + look.dither * (BAYER[(ix & 1) + 2 * (iy & 1)] / 4 - 0.375), look);
    return clamp(t1, t0 - 1, t0 + 1);   // fur noise moves a pixel one shade at most
  }

  function bbox(s) {
    if (s.k === 'cap') { const m = Math.max(s.ra, s.rb); return [Math.min(s.a[0], s.b[0]) - m, Math.min(s.a[1], s.b[1]) - m, Math.max(s.a[0], s.b[0]) + m, Math.max(s.a[1], s.b[1]) + m]; }
    if (s.k === 'ell') { const m = Math.max(s.rx, s.ry); return [s.c[0] - m, s.c[1] - m, s.c[0] + m, s.c[1] + m]; }
    const xs = s.p.map(q => q[0]), ys = s.p.map(q => q[1]);
    return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
  }

  // Rasterise a list of shapes. Returns per-pixel family/tone/group.
  function raster(list, look, fs, paintFn) {
    paintFn = paintFn || paint;
    const { W, H: Hh, ox, gy } = fs, R = look.R, N = W * Hh;
    const fam = new Int8Array(N).fill(-1), tone = new Int8Array(N), grp = new Int8Array(N).fill(-1), edg = new Int8Array(N);
    for (const s of list) {
      const b = bbox(s);
      const x0 = clamp(Math.floor(ox + b[0] * R), 0, W - 1), x1 = clamp(Math.ceil(ox + b[2] * R), 0, W - 1);
      const y0 = clamp(Math.floor(gy - b[3] * R), 0, Hh - 1), y1 = clamp(Math.ceil(gy - b[1] * R), 0, Hh - 1);
      const anc = s.k === 'cap' ? s.a : s.k === 'ell' ? s.c : s.p[0];
      const ax = Math.round(anc[0] * R), ay = Math.round(anc[1] * R);
      for (let iy = y0; iy <= y1; iy++) for (let ix = x0; ix <= x1; ix++) {
        const x = (ix + 0.5 - ox) / R, y = (gy - iy - 0.5) / R;
        if (!hit(s, x, y)) continue;
        const nz = hash(ix - ox - ax, iy + ay);
        const i = iy * W + ix, fm = s.grp === 9 ? paint(s, x, y, look, nz) : paintFn(s, x, y, look, nz);
        let tn = toneOf(H.lit, look, s.part === 'coco' ? nz * 2.5 : nz, ix, iy);
        if (s.far) tn = Math.max(0, tn - 1);
        fam[i] = fm; tone[i] = tn; grp[i] = s.grp; edg[i] = s.noEdge ? 0 : 1;
      }
    }
    return { fam, tone, grp, edg };
  }

  function toRGBA(rs, look, fs, pal) {
    const { W, H: Hh } = fs, N = W * Hh, out = new Uint8ClampedArray(N * 4);
    const { fam, tone, grp, edg } = rs;
    // lone pixels of a different colour read as dirt: absorb them
    for (let iy = 1; iy < Hh - 1; iy++) for (let ix = 1; ix < W - 1; ix++) {
      const i = iy * W + ix, a = fam[i - 1];
      if (a < 0 || fam[i] === a || fam[i + 1] !== a || fam[i - W] !== a || fam[i + W] !== a) continue;
      if (fam[i] === F.nose || fam[i] === F.tongue) continue;
      fam[i] = a;
    }
    // separate near legs from the body behind them
    if (look.detail >= 1) {
      const t2 = tone.slice(), drop = pal.edgeDrop;
      for (let iy = 0; iy < Hh; iy++) for (let ix = 1; ix < W - 1; ix++) {
        const i = iy * W + ix, g = grp[i];
        if ((g !== 1 && g !== 2 && g !== 6) || !edg[i] || iy < 1 || iy >= Hh - 1) continue;
        for (const j of [i - 1, i + 1, i - W, i + W]) if (grp[j] >= 0 && grp[j] !== g && grp[j] !== 9) { t2[i] = Math.max(0, tone[i] - drop); break; }
      }
      tone.set(t2);
    }
    for (let i = 0; i < N; i++) {
      if (fam[i] < 0) continue;
      const c = pal.fam[fam[i]][tone[i]];
      out[i * 4] = c[0]; out[i * 4 + 1] = c[1]; out[i * 4 + 2] = c[2]; out[i * 4 + 3] = 255;
    }
    // outline
    const o = pal.outline;
    for (let iy = 0; iy < Hh; iy++) for (let ix = 0; ix < W; ix++) {
      const i = iy * W + ix;
      if (fam[i] >= 0) continue;
      let nb = -1;
      if (ix > 0 && fam[i - 1] >= 0) nb = i - 1;
      else if (ix < W - 1 && fam[i + 1] >= 0) nb = i + 1;
      else if (iy > 0 && fam[i - W] >= 0) nb = i - W;
      else if (iy < Hh - 1 && fam[i + W] >= 0) nb = i + W;
      if (nb < 0) continue;
      let c = o;
      if (look.outline === 'soft') { const d = pal.fam[fam[nb]][0]; c = [(d[0] + o[0]) >> 1, (d[1] + o[1]) >> 1, (d[2] + o[2]) >> 1]; }
      out[i * 4] = c[0]; out[i * 4 + 1] = c[1]; out[i * 4 + 2] = c[2]; out[i * 4 + 3] = 255;
    }
    return out;
  }

  function put(out, fs, ix, iy, c) {
    if (ix < 0 || iy < 0 || ix >= fs.W || iy >= fs.H) return;
    const i = (iy * fs.W + ix) * 4; out[i] = c[0]; out[i + 1] = c[1]; out[i + 2] = c[2]; out[i + 3] = 255;
  }

  function drawEyes(out, S, B, look, fs, pal) {
    const hs = B.head * (S.headFront ? 1.15 : 1), ang = S.headFront ? S.tilt : S.headRot;
    const pts = S.headFront ? [[-0.08, 0.04], [0.08, 0.04]] : [[0.095, 0.04]];
    for (const p of pts) {
      const w = add(S.head, rot(mul(p, hs), ang));
      const ix = Math.floor(fs.ox + w[0] * look.R), iy = Math.floor(fs.gy - w[1] * look.R);
      if (S.eyes > 0.5) {
        put(out, fs, ix, iy, pal.eye);
        if (look.detail >= 2) put(out, fs, S.headFront ? ix : ix - 1, S.headFront ? iy - 1 : iy, pal.iris);
        if (look.detail >= 2 && S.headFront) put(out, fs, ix, iy + 1, pal.eye);
      } else {
        put(out, fs, ix, iy, pal.eye); put(out, fs, ix - 1, iy, pal.eye);
        if (look.detail >= 2) put(out, fs, ix + 1, iy, pal.eye);
      }
    }
  }

  // ---------------------------------------------------------------- public
  const palCache = new WeakMap();
  // Each species brings its own body sliders, poses, shapes, markings and eyes;
  // rasterising, shading and outlines are shared.
  const SPECIES = {
    dog: { anims: ANIMS, palette: ALEX, body: look => look.body, shapes, paint, eyes: drawEyes }
  };
  function palFor(coat) {
    let pal = palCache.get(coat); if (!pal) { pal = compilePalette(coat); palCache.set(coat, pal); }
    return pal;
  }
  function render(S, look, opts) {
    opts = opts || {};
    const sp = SPECIES[S.species || 'dog'], B = sp.body(look), fs = frameSize(look);
    const pal = palFor(opts.palette || sp.palette);
    const list = sp.shapes(S, B, look);
    if (opts.collar) for (const s of list) if (s.part === 'neck') s.collar = true;
    const rs = raster(list, look, fs, sp.paint);
    const out = toRGBA(rs, look, fs, pal);
    sp.eyes(out, S, B, look, fs, pal);
    return { w: fs.W, h: fs.H, ox: fs.ox, gy: fs.gy, data: out };
  }

  function renderToy(type, look, opts) {
    const fs = { W: Math.ceil(0.3 * look.R) + 4, H: Math.ceil(0.25 * look.R) + 4 };
    fs.ox = Math.floor(fs.W / 2); fs.gy = fs.H - 2;
    const rad = type === 'coconut' ? 0.10 : type === 'football' ? 0.085 : 0.072;
    const s = toyShape(type, [0, Math.max(rad, look.minR / look.R) * 0.95]);
    const pal = palFor((opts && opts.palette) || ALEX);
    const rs = raster([s], look, fs);
    return { w: fs.W, h: fs.H, ox: fs.ox, gy: fs.gy, data: toRGBA(rs, look, fs, pal) };
  }

  ANIMS.walk.speed = walkSpeed;
  function anim(species, name) { return SPECIES[species || 'dog'].anims[name]; }
  function pose(name, look, t, opts) {
    opts = opts || {};
    const sp = SPECIES[opts.species || 'dog'], S = sp.anims[name].fn(sp.body(look), t, opts);
    S.species = opts.species || 'dog';
    return S;
  }
  // units per second while this animation plays; move(p) shapes it over one loop
  function speed(look, species, name) {
    const sp = SPECIES[species || 'dog'], A = sp.anims[name];
    return A && A.speed ? A.speed(sp.body(look)) : 0;
  }

  const util = { add, sub, mul, len, rot, clamp, mix, TAU, PAW_Y, cap, ell, tri, F, H, put, lerpSkel };
  return { LOOKS, ANIMS, ALEX, BODY, SPECIES, util, anim, pose, speed, lerpSkel, render, renderToy, walkSpeed, frameSize };
});
