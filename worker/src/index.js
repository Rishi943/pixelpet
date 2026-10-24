// PixelPet upload: pet photos from the page go to a private GitHub repo as
// submissions/<id>/ (photos + info.md), with an issue labelled "new".
//
//   POST /start   {turnstile}                      -> {session}   (bot check)
//   POST /photo   base64 JPEG/PNG/WebP, one photo  -> {sha}       (a git blob)
//   POST /submit  {session, name, species, notes, consent, photos: [{sha, type}]}
//
// Photos travel as base64 one per request and are passed through untouched,
// so each request stays well inside the free plan's CPU time. The page
// shrinks them first (and so drops their location data).
//
// Secrets: GITHUB_TOKEN (contents + issues on REPO only), TURNSTILE_SECRET,
// SESSION_KEY. Vars: REPO, ALLOW_ORIGIN (comma-separated), TURNSTILE_HOSTNAME.

const MAX_PHOTOS = 5;
const MAX_PHOTO_B64 = 2_800_000; // ~2 MB of image
const SESSION_SECS = 15 * 60;
const SPECIES = ["Dog", "Cat", "Rabbit", "Something else"];
const EXT = { jpeg: "jpg", png: "png", webp: "webp" };

export default {
  async fetch(req, env) {
    const origin = req.headers.get("Origin") || "";
    const ok = allowed(origin, env);
    const cors = { "Access-Control-Allow-Origin": ok ? origin : "null", Vary: "Origin" };
    if (req.method === "OPTIONS") {
      return new Response(null, {
        headers: { ...cors, "Access-Control-Allow-Methods": "POST", "Access-Control-Allow-Headers": "Content-Type, X-Session, X-Index", "Access-Control-Max-Age": "86400" },
      });
    }
    const path = new URL(req.url).pathname;
    if (req.method !== "POST" || !["/start", "/photo", "/submit"].includes(path)) return reply({ error: "Not found." }, 404, cors);
    if (!ok) return reply({ error: "Uploads only come from the PixelPet page." }, 403, cors);
    try {
      if (path === "/start") return await start(req, env, cors);
      if (path === "/photo") return await photo(req, env, cors);
      return await submit(req, env, cors);
    } catch (e) {
      console.error(e);
      return reply({ error: "Something went wrong on our side. Please try again later." }, 500, cors);
    }
  },
};

function allowed(origin, env) {
  return (env.ALLOW_ORIGIN || "").split(",").map((s) => s.trim()).includes(origin);
}

function reply(body, status, cors) {
  return new Response(JSON.stringify(body), { status, headers: { ...cors, "Content-Type": "application/json" } });
}

// ---------------------------------------------------------------- the steps

async function start(req, env, cors) {
  const { turnstile } = await req.json().catch(() => ({}));
  const form = new FormData();
  form.append("secret", env.TURNSTILE_SECRET);
  form.append("response", String(turnstile || ""));
  const ip = req.headers.get("CF-Connecting-IP");
  if (ip) form.append("remoteip", ip);
  const r = await fetch("https://challenges.cloudflare.com/turnstile/v0/siteverify", { method: "POST", body: form });
  const v = await r.json();
  if (!v.success || (env.TURNSTILE_HOSTNAME && v.hostname !== env.TURNSTILE_HOSTNAME)) {
    return reply({ error: "The robot check didn't pass. Reload the page and try again." }, 403, cors);
  }
  return reply({ session: await sign(env, { exp: Math.floor(Date.now() / 1000) + SESSION_SECS, id: submissionId() }) }, 200, cors);
}

async function photo(req, env, cors) {
  const s = await verify(env, req.headers.get("X-Session"));
  const i = Number(req.headers.get("X-Index"));
  if (!s) return reply({ error: "This upload timed out. Reload the page and try again." }, 403, cors);
  if (!(i >= 1 && i <= MAX_PHOTOS)) return reply({ error: `Up to ${MAX_PHOTOS} photos.` }, 400, cors);
  const b64 = (await req.text()).trim();
  if (b64.length > MAX_PHOTO_B64) return reply({ error: "That photo is too big." }, 413, cors);
  const type = imageType(b64);
  // GitHub rejects bad base64 itself; only make sure it can't break out of the JSON string
  if (!type || b64.includes('"') || b64.includes("\\")) return reply({ error: "Only JPG, PNG or WebP photos, please." }, 415, cors);
  const blob = await gh(env, "POST", "/git/blobs", `{"encoding":"base64","content":"${b64}"}`);
  return reply({ sha: blob.sha, type }, 200, cors);
}

async function submit(req, env, cors) {
  const body = await req.json().catch(() => null);
  const s = body && (await verify(env, body.session));
  if (!s) return reply({ error: "This upload timed out. Reload the page and try again." }, 403, cors);
  const name = clean(body.name, 40).replace(/[[\]()<>`*_#|]/g, "");
  const species = SPECIES.includes(body.species) ? body.species : null;
  const notes = clean(body.notes, 1000).replace(/`/g, "'");
  const photos = Array.isArray(body.photos) ? body.photos.slice(0, MAX_PHOTOS) : [];
  if (!name || !species) return reply({ error: "Please give your pet's name and what kind of animal they are." }, 400, cors);
  if (body.consent !== true) return reply({ error: "Please tick the box to say the photos are okay to use." }, 400, cors);
  if (!photos.length || !photos.every((p) => /^[0-9a-f]{40}$/.test(p.sha) && EXT[p.type])) {
    return reply({ error: "Please add at least one photo." }, 400, cors);
  }

  const dir = `submissions/${s.id}`;
  const files = photos.map((p, k) => ({ path: `${dir}/photo-${k + 1}.${EXT[p.type]}`, sha: p.sha }));
  const view = (p) => `https://github.com/${env.REPO}/blob/main/${p}`;
  const info = [
    `# ${name}`,
    "",
    `- Species: ${species}`,
    `- Sent: ${new Date().toISOString()}`,
    `- Photos: ${files.length}`,
    "",
    "## Notes from the sender",
    "",
    "```text",
    notes || "(none)",
    "```",
    "",
    ...files.map((f, k) => `![photo ${k + 1}](${view(f.path)}?raw=true)`),
    "",
  ].join("\n");
  const infoBlob = await gh(env, "POST", "/git/blobs", JSON.stringify({ encoding: "utf-8", content: info }));
  await commit(env, [...files, { path: `${dir}/info.md`, sha: infoBlob.sha }], `submission: ${name} (${species})`);
  await gh(env, "POST", "/issues", JSON.stringify({
    title: `${name} (${species})`,
    labels: ["new"],
    body: `${info}\nFolder: [${dir}](${view(dir)})\n`,
  }));
  return reply({ ok: true }, 200, cors);
}

// ---------------------------------------------------------------- helpers

function clean(v, max) {
  return String(v ?? "").replace(/[\u0000-\u0009\u000B-\u001F\u007F]/g, "").trim().slice(0, max);
}

function submissionId() {
  const d = new Date().toISOString().slice(0, 10);
  const r = [...crypto.getRandomValues(new Uint8Array(4))].map((b) => b.toString(16).padStart(2, "0")).join("");
  return `${d}-${r}`;
}

/** The image type from the first bytes of base64 data, or null. */
function imageType(b64) {
  let head;
  try {
    head = atob(b64.slice(0, 24));
  } catch {
    return null;
  }
  const c = (i) => head.charCodeAt(i);
  if (c(0) === 0xff && c(1) === 0xd8 && c(2) === 0xff) return "jpeg";
  if (c(0) === 0x89 && head.slice(1, 4) === "PNG") return "png";
  if (head.slice(0, 4) === "RIFF" && head.slice(8, 12) === "WEBP") return "webp";
  return null;
}

async function gh(env, method, path, body) {
  const r = await fetch(`${env.GITHUB_API || "https://api.github.com"}/repos/${env.REPO}${path}`, {
    method,
    body,
    headers: {
      Authorization: `Bearer ${env.GITHUB_TOKEN}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      "User-Agent": "pixelpet-upload",
      "Content-Type": "application/json",
    },
  });
  if (!r.ok) throw new Error(`GitHub ${method} ${path}: ${r.status} ${await r.text()}`);
  return r.json();
}

/** One commit on main adding `files` (already blobs), retried if main moved. */
async function commit(env, files, message) {
  for (let attempt = 0; attempt < 4; attempt++) {
    const head = (await gh(env, "GET", "/git/ref/heads/main")).object.sha;
    const base = (await gh(env, "GET", `/git/commits/${head}`)).tree.sha;
    const tree = await gh(env, "POST", "/git/trees", JSON.stringify({ base_tree: base, tree: files.map((f) => ({ path: f.path, mode: "100644", type: "blob", sha: f.sha })) }));
    const c = await gh(env, "POST", "/git/commits", JSON.stringify({ message, tree: tree.sha, parents: [head] }));
    try {
      await gh(env, "PATCH", "/git/refs/heads/main", JSON.stringify({ sha: c.sha }));
      return;
    } catch (e) {
      if (!String(e).includes(": 422")) throw e; // someone else committed first: go again
    }
  }
  throw new Error("main kept moving");
}

// a session: base64url(JSON) + "." + base64url(HMAC-SHA256)
const enc = new TextEncoder();
const b64url = (bytes) => btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const unb64url = (s) => Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/")), (ch) => ch.charCodeAt(0));

async function key(env) {
  return crypto.subtle.importKey("raw", enc.encode(env.SESSION_KEY), { name: "HMAC", hash: "SHA-256" }, false, ["sign", "verify"]);
}

async function sign(env, payload) {
  const p = b64url(enc.encode(JSON.stringify(payload)));
  return `${p}.${b64url(await crypto.subtle.sign("HMAC", await key(env), enc.encode(p)))}`;
}

async function verify(env, token) {
  const [p, mac] = String(token || "").split(".");
  if (!p || !mac) return null;
  let good = false;
  try {
    good = await crypto.subtle.verify("HMAC", await key(env), unb64url(mac), enc.encode(p));
  } catch {
    return null;
  }
  if (!good) return null;
  const s = JSON.parse(new TextDecoder().decode(unb64url(p)));
  return s.exp > Date.now() / 1000 ? s : null;
}
