# Leaderboard server

A Cloudflare Worker (`src/index.js`) in front of one D1 table (`schema.sql`).
The game (`src/leaderboard.rs`) submits each player's score whenever it
changes and reads the board when its screen opens.

| | |
|---|---|
| `POST /score` | `{id, name, rad, species, animal}` → `{rank}` |
| `GET /top?id=<player id>` | `{top: [50 rows], me: <your row if outside the top>, players}` |

Players are ranked by RAD, then by species found on any Earth; on a tie,
whoever got there first. `id` is random, made by the game on first launch,
and never sent back to anyone else.

## Cheating

The game runs on the player's device, so any score can be forged. The
server checks what it can with its own clock, which no one else controls:

- a score can't grow faster than real waiting allows (species and RAD per
  hour since the player's last accepted submit);
- a first submit (a whole save from before the leaderboard) is only capped:
  up to `TAXA` species and 1000 RAD;
- names are 3 to 16 of `A-Z a-z 0-9 space _ -`;
- submits less than 5 seconds apart are refused.

`TAXA` in `wrangler.toml` is the number of taxa in `src/tree.rs`: bump it
when the tree grows, or new finds will be refused.

## One-time setup

Needs Node.js and a Cloudflare account (the free plan is enough).

```sh
cd server
npm install
npx wrangler login                          # opens the browser
npx wrangler d1 create ascendio-scores      # prints a database_id
#   -> paste it into wrangler.toml
npx wrangler d1 execute ascendio-scores --remote --file schema.sql
npx wrangler deploy                         # prints the Worker's URL
```

Then put that URL in `DEFAULT_URL` in `src/leaderboard.rs` and rebuild the
game (`make apk`, `make web`).

`schema.sql` starts with `DROP TABLE`: running it again wipes the board.

## Later

```sh
npx wrangler deploy                                     # after editing src/index.js
npx wrangler d1 execute ascendio-scores --remote \
  --command "SELECT name, rad, species FROM scores ORDER BY rad DESC, species DESC LIMIT 20"
npx wrangler d1 execute ascendio-scores --remote \
  --command "DELETE FROM scores WHERE name = 'SomeCheater'"
```

## Locally

No account needed: `wrangler dev` runs the Worker and a local D1 on this
machine.

```sh
npx wrangler d1 execute ascendio-scores --local --file schema.sql
npx wrangler dev --port 8787
# in the repo root, a desktop build that talks to it:
ASCENDIO_LEADERBOARD_URL=http://127.0.0.1:8787 cargo run --release
```

Desktop builds send their requests with `curl`, which must be installed.
Dev runs (`ASCENDIO_SCRATCH`, `ASCENDIO_TIME_SCALE`, `ASCENDIO_AUTOPLAY`,
screenshots, demos) never submit.
