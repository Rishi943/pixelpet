// Renders every look x pose into one PNG so the art can be checked by eye.
// usage: node sheet.js out.png
const zlib = require('zlib');
const fs = require('fs');
const Rig = require('./rig.js');

function png(w, h, rgba) {
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) Buffer.from(rgba.buffer, y * w * 4, w * 4).copy(raw, y * (w * 4 + 1) + 1);
  const chunk = (type, data) => {
    const b = Buffer.alloc(12 + data.length);
    b.writeUInt32BE(data.length, 0); b.write(type, 4); data.copy(b, 8);
    b.writeUInt32BE(zlib.crc32(Buffer.concat([Buffer.from(type), data])) >>> 0, 8 + data.length);
    return b;
  };
  const ih = Buffer.alloc(13); ih.writeUInt32BE(w, 0); ih.writeUInt32BE(h, 4); ih[8] = 8; ih[9] = 6;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ih), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

const cells = [
  ['stand', 0], ['walk', 0], ['walk', 0.225], ['walk', 0.45], ['walk', 0.675],
  ['sit', 0], ['tilt', 1.5], ['tilt', 5.5], ['hop', 0.15], ['hop', 0.5], ['sleep', 1], ['walk', 0.1, { toy: 'ball' }], ['stand', 0.3, { toy: 'coconut' }]
];
const looks = Object.values(Rig.LOOKS);
const scale = +(process.argv[3] || 1);
const cw = Math.max(...looks.map(l => Rig.frameSize(l).W * l.zoom)) * scale, ch = Math.max(...looks.map(l => Rig.frameSize(l).H * l.zoom)) * scale;
const W = cw * cells.length, Hh = ch * looks.length, img = new Uint8ClampedArray(W * Hh * 4);
for (let i = 0; i < W * Hh; i++) { img[i * 4] = 214; img[i * 4 + 1] = 220; img[i * 4 + 2] = 228; img[i * 4 + 3] = 255; }
looks.forEach((look, r) => cells.forEach(([name, t, o], c) => {
  const S = Rig.pose(name, look, t, o);
  const f = Rig.render(S, look, { collar: false });
  const z = look.zoom * scale, x0 = c * cw + Math.floor((cw - f.w * z) / 2), y0 = r * ch + (ch - f.h * z);
  for (let y = 0; y < f.h * z; y++) for (let x = 0; x < f.w * z; x++) {
    const si = (Math.floor(y / z) * f.w + Math.floor(x / z)) * 4;
    if (!f.data[si + 3]) continue;
    const di = ((y0 + y) * W + x0 + x) * 4;
    img[di] = f.data[si]; img[di + 1] = f.data[si + 1]; img[di + 2] = f.data[si + 2];
  }
  const gyPix = r * ch + (ch - f.h * z) + (f.gy + 1) * z;   // ground line
  for (let x = c * cw; x < (c + 1) * cw; x++) { const di = (Math.min(gyPix, Hh - 1) * W + x) * 4; img[di] = 120; img[di + 1] = 130; img[di + 2] = 145; }
}));
fs.writeFileSync(process.argv[2], png(W, Hh, img));
console.log('wrote', process.argv[2], W + 'x' + Hh);
