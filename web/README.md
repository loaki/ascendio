# Web build

`gl.js` is miniquad 0.4.11's loader, copied verbatim from the crate source so
it can never drift from the version the wasm was built against:

    cp ~/.cargo/registry/src/*/miniquad-0.4.11/js/gl.js web/gl.js

It is the only JavaScript needed. macroquad 0.4.16 has `default = []`, so the
audio feature is off and none of the extra `mq_js_bundle.js` parts apply.

`ascendio.wasm` is a build artifact and is gitignored; `make web` produces it.
