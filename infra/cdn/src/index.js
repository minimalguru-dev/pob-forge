// PoE Forge CDN: serves tree art, item art and game data from an R2 bucket,
// and the updater manifest from the latest GitHub release.
//
// Paths the app asks for:
//   tree/<sha256>.<ext>                hashed tree art (immutable)
//   data/<sha256>                      hashed game data (immutable)
//   maps/<game>/latest.json            item-art map (changes with leagues)
//   <game>/Art/...webp?v=<tag>         item art, versioned by tag
//   update/latest.json                 updater manifest
//
// With ORIGIN set, a file missing from the bucket is fetched from ORIGIN once,
// stored, and served; leave ORIGIN empty to serve only what is in the bucket.

const YEAR = "public, max-age=31536000, immutable";
const DAY = "public, max-age=86400";
/** Mutable files are re-fetched from ORIGIN when the stored copy is older than this. */
const REFRESH_MS = 60 * 60 * 1000;

export default {
  async fetch(request, env, ctx) {
    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method not allowed", { status: 405, headers: { Allow: "GET, HEAD" } });
    }
    const url = new URL(request.url);
    let path;
    try {
      path = decodeURIComponent(url.pathname).replace(/^\/+/, "");
    } catch {
      return notFound();
    }
    if (!path || path.split("/").some((p) => p === "" || p === "." || p === "..") || path.includes("\\")) return notFound();

    if (path === "update/latest.json") return updateManifest(env);

    const tag = url.searchParams.get("v");
    if (tag !== null && !/^[0-9a-f]{8}$/i.test(tag)) return notFound();
    const key = tag ? `${path}?v=${tag}` : path;
    const mutable = /^maps\/[^/]+\/latest\.json$/.test(path);
    const cache = path.startsWith("tree/") || path.startsWith("data/") || tag ? YEAR : mutable ? "no-cache" : DAY;

    const stored = await env.BUCKET.get(key, { onlyIf: request.headers });
    const stale = stored && mutable && Date.now() - stored.uploaded.getTime() > REFRESH_MS;
    if (stored && !stale) return fromBucket(stored, request, cache);

    const pulled = env.ORIGIN ? await pull(env, ctx, key, path, tag) : null;
    if (pulled) return withHeaders(pulled, request, cache);
    // The origin failed, but a stale copy is better than nothing.
    if (stored) return fromBucket(stored, request, cache);
    return notFound();
  },
};

function fromBucket(object, request, cache) {
  const headers = new Headers();
  object.writeHttpMetadata(headers);
  headers.set("ETag", object.httpEtag);
  headers.set("Cache-Control", cache);
  headers.set("Access-Control-Allow-Origin", "*");
  // `onlyIf` matched the client's If-None-Match: R2 returns the object without a body.
  if (!("body" in object)) return new Response(null, { status: 304, headers });
  return new Response(request.method === "HEAD" ? null : object.body, { headers });
}

async function pull(env, ctx, key, path, tag) {
  const origin = env.ORIGIN.replace(/\/+$/, "");
  const res = await fetch(`${origin}/${path.split("/").map(encodeURIComponent).join("/")}${tag ? `?v=${tag}` : ""}`);
  if (!res.ok) return null;
  const body = await res.arrayBuffer();
  const contentType = res.headers.get("Content-Type") ?? guessType(path);
  ctx.waitUntil(env.BUCKET.put(key, body, { httpMetadata: { contentType } }));
  return { body, contentType };
}

function withHeaders({ body, contentType }, request, cache) {
  return new Response(request.method === "HEAD" ? null : body, {
    headers: { "Content-Type": contentType, "Cache-Control": cache, "Access-Control-Allow-Origin": "*" },
  });
}

async function updateManifest(env) {
  if (!env.UPDATE_REPO) return notFound();
  const res = await fetch(`https://github.com/${env.UPDATE_REPO}/releases/latest/download/latest.json`, {
    cf: { cacheTtl: 300, cacheEverything: true },
  });
  if (!res.ok) return new Response("No release yet", { status: 404 });
  return new Response(res.body, { headers: { "Content-Type": "application/json", "Cache-Control": "public, max-age=300" } });
}

function guessType(path) {
  if (path.endsWith(".webp")) return "image/webp";
  if (path.endsWith(".png")) return "image/png";
  if (path.endsWith(".jpg")) return "image/jpeg";
  if (path.endsWith(".json")) return "application/json";
  return "application/octet-stream";
}

function notFound() {
  return new Response("Not found", { status: 404, headers: { "Cache-Control": "no-store" } });
}
