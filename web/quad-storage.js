var ctx = null;
var memory;

params_set_mem = function (wasm_memory, _wasm_exports) {
    memory = wasm_memory;
    ctx = {};
}

// localStorage throws when the browser blocks it (privacy settings, some
// embedded views) or it is full; an exception here would unwind into the
// wasm and stop the frame loop, so storage that fails acts empty instead
// (src/save.rs never panics on a missing value).
function quad_storage_try(f, otherwise) {
    try {
        return f();
    } catch (e) {
        console.warn("quad-storage:", e);
        return otherwise;
    }
}

params_register_js_plugin = function (importObject) {
    importObject.env.quad_storage_length = function () {
        return quad_storage_try(function () { return localStorage.length; }, 0);
    }
    importObject.env.quad_storage_has_key = function (i) {
        return quad_storage_try(function () { return +(localStorage.key(i) != null); }, 0);
    }
    importObject.env.quad_storage_key = function (i) {
        return js_object(quad_storage_try(function () { return localStorage.key(i); }, "") || "");
    }
    importObject.env.quad_storage_has_value = function (key) {
        var k = get_js_object(key);
        return quad_storage_try(function () { return +(localStorage.getItem(k) != null); }, 0);
    }
    importObject.env.quad_storage_get = function (key) {
        var k = get_js_object(key);
        return js_object(quad_storage_try(function () { return localStorage.getItem(k); }, "") || "");
    }
    importObject.env.quad_storage_set = function (key, value) {
        var k = get_js_object(key);
        var v = get_js_object(value);
        quad_storage_try(function () { localStorage.setItem(k, v); });
    }
    importObject.env.quad_storage_remove = function (key) {
        var k = get_js_object(key);
        quad_storage_try(function () { localStorage.removeItem(k); });
    }
    importObject.env.quad_storage_clear = function () {
        quad_storage_try(function () { localStorage.clear(); });
    }
}

miniquad_add_plugin({
    register_plugin: params_register_js_plugin,
    on_init: params_set_mem,
    name: "quad_storage",
    version: "0.1.4"
});
