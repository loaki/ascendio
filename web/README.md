# Web build

`index.html` loads three vendored scripts and one of the game's own, in this
order, before the wasm:

| File | Where it comes from |
|---|---|
| `gl.js` | miniquad 0.4.11's loader, copied verbatim so it never drifts from the version the wasm was built against: `cp ~/.cargo/registry/src/*/miniquad-0.4.11/js/gl.js web/gl.js` |
| `sapp_jsutils.js` | the `sapp-jsutils` crate's `js/` directory (not in its crates.io tarball) |
| `quad-storage.js` | the `quad-storage` crate's `js/` directory: the save plugin (`localStorage`) |
| `ascendio.js` | ours (`src/net.rs`): `fetch` for the leaderboard, `prompt` for the player's name |

macroquad 0.4.16 has `default = []`, so the audio feature is off and none of
the extra `mq_js_bundle.js` parts apply.

`ascendio.wasm` is a build artifact and is gitignored; `make web` produces it.
