// The Ascendio leaderboard: a Cloudflare Worker in front of one D1 table
// (schema.sql). Players are ranked by RAD, then by species found on any
// Earth; on a tie, whoever got there first.
//
//   POST /score  {id, name, rad, species, animal, reset?}  -> {rank, rad, species}
//   GET  /top?id=<player id>                      -> {top, me, players}
//
// The game is client-side, so a score can always be forged. What the server
// can check is time: every genome takes hours of real waiting, measured here
// with the server's clock, so a score can only grow so fast.

const TOP = 50;
const ID = /^[0-9a-f]{32}$/;
const NAME = /^[A-Za-z0-9 _-]{3,16}$/;
/// Fastest a real player can progress, per hour since their last submit,
/// with room to spare: the shortest wait (2h, -30% from keystones, halved by
/// Tailwind) is 0.7h and a genome holds at most ~10 cards. The slack covers
/// a first wait of the tutorial, which takes only minutes.
const SPECIES_PER_HOUR = 20;
const SPECIES_SLACK = 20;
const RAD_PER_HOUR = 2;
/// A new ID has no past to check against, so its first score is cut down to
/// what a new player can have: no RAD (that takes ending an Earth, far down
/// the tree), and the Urmetazoan, two tutorial genomes of 3 cards and a few
/// first waits (2 to 5 cards each), with room to spare. A save from before
/// the leaderboard climbs from there at the per-hour rates above, so a made
/// up ID can't take the top at once and a real long-time player catches up
/// within a day.
const FIRST_RAD_MAX = 0;
const FIRST_SPECIES_MAX = 20;
/// Submits closer together than this are refused.
const MIN_GAP_MS = 5000;

const CORS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type",
};

function json(body, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...CORS },
  });
}

function fail(message, status) {
  return json({ error: message }, status);
}

/// Players ranked strictly above a row with these values.
const AHEAD = `SELECT COUNT(*) AS n FROM scores
  WHERE rad > ?1 OR (rad = ?1 AND species > ?2)
     OR (rad = ?1 AND species = ?2 AND reached < ?3)`;

async function rankOf(db, row) {
  const { n } = await db.prepare(AHEAD).bind(row.rad, row.species, row.reached).first();
  return n + 1;
}

async function submit(req, env) {
  let body;
  try {
    body = await req.json();
  } catch {
    return fail("bad json", 400);
  }
  const { id, rad, species, animal } = body;
  const name = typeof body.name === "string" ? body.name.trim() : "";
  const taxa = Number(env.TAXA);
  const int = (v, lo, hi) => Number.isInteger(v) && v >= lo && v <= hi;
  if (typeof id !== "string" || !ID.test(id)) return fail("bad id", 400);
  if (!NAME.test(name)) return fail("bad name", 400);
  if (!int(species, 1, taxa) || !int(animal, 0, taxa - 1) || !int(rad, 0, 100000)) {
    return fail("bad score", 400);
  }

  const db = env.DB;
  const now = Date.now();
  // "reset": the player started over, so their old row is forgotten and the
  // score is treated like a new ID's first.
  const old =
    body.reset === true
      ? null
      : await db.prepare("SELECT * FROM scores WHERE player_id = ?").bind(id).first();
  if (old && now - old.updated < MIN_GAP_MS) return fail("too fast", 429);

  // Best scores only ever grow, and no faster than the per-hour rates: a
  // bigger jump is cut down rather than refused, so a long-time player
  // climbs to their real score over a few submits. The name and the animal
  // follow the latest.
  let best;
  if (old) {
    const hours = (now - old.updated) / 3.6e6;
    const maxSpecies = old.species + SPECIES_SLACK + Math.floor(hours * SPECIES_PER_HOUR);
    const maxRad = old.rad + 1 + Math.floor(hours * RAD_PER_HOUR);
    best = {
      rad: Math.max(old.rad, Math.min(rad, maxRad)),
      species: Math.max(old.species, Math.min(species, maxSpecies, taxa)),
    };
  } else {
    best = { rad: Math.min(rad, FIRST_RAD_MAX), species: Math.min(species, FIRST_SPECIES_MAX) };
  }
  const improved = !old || best.rad > old.rad || best.species > old.species;
  const reached = improved ? now : old.reached;
  await db
    .prepare(
      `INSERT INTO scores (player_id, name, rad, species, animal, reached, updated)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
       ON CONFLICT (player_id) DO UPDATE SET
         name = ?2, rad = ?3, species = ?4, animal = ?5, reached = ?6, updated = ?7`
    )
    .bind(id, name, best.rad, best.species, animal, reached, now)
    .run();
  return json({ rank: await rankOf(db, { ...best, reached }), rad: best.rad, species: best.species });
}

async function top(url, env) {
  const db = env.DB;
  const id = url.searchParams.get("id") || "";
  const { results } = await db
    .prepare(
      `SELECT player_id, name, rad, species, animal FROM scores
       ORDER BY rad DESC, species DESC, reached ASC LIMIT ?`
    )
    .bind(TOP)
    .all();
  const top = results.map((r, i) => ({
    rank: i + 1,
    name: r.name,
    rad: r.rad,
    species: r.species,
    animal: r.animal,
    me: r.player_id === id,
  }));
  let me = null;
  if (ID.test(id) && !top.some((r) => r.me)) {
    const row = await db.prepare("SELECT * FROM scores WHERE player_id = ?").bind(id).first();
    if (row) {
      me = {
        rank: await rankOf(db, row),
        name: row.name,
        rad: row.rad,
        species: row.species,
        animal: row.animal,
        me: true,
      };
    }
  }
  const { n } = await db.prepare("SELECT COUNT(*) AS n FROM scores").first();
  return json({ top, me, players: n });
}

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method === "OPTIONS") return new Response(null, { status: 204, headers: CORS });
    if (url.pathname === "/score" && req.method === "POST") return submit(req, env);
    if (url.pathname === "/top" && req.method === "GET") return top(url, env);
    return fail("not found", 404);
  },
};
