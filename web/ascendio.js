// The game's own browser calls (src/net.rs): fetch for the leaderboard and
// prompt for the player's name. Needs sapp_jsutils.js for the strings.
(function () {
    var requests = {};
    var next = 1;
    var promptText = "";

    function register(importObject) {
        // Returns an id to poll; status 0 while in flight, 1 if nothing
        // answered, else the HTTP status.
        importObject.env.ascendio_http = function (method, url, body) {
            var id = next++;
            var text = get_js_object(body);
            var init = { method: get_js_object(method) };
            if (text.length > 0) {
                init.headers = { "Content-Type": "application/json" };
                init.body = text;
            }
            requests[id] = { status: 0, body: "" };
            fetch(get_js_object(url), init)
                .then(function (r) {
                    return r.text().then(function (t) {
                        // 0 is reserved for "in flight".
                        requests[id] = { status: r.status || 1, body: t };
                    });
                })
                .catch(function () {
                    requests[id] = { status: 1, body: "" };
                });
            return id;
        };
        importObject.env.ascendio_http_status = function (id) {
            return requests[id] ? requests[id].status : 1;
        };
        importObject.env.ascendio_http_take = function (id) {
            var body = requests[id] ? requests[id].body : "";
            delete requests[id];
            return js_object(body);
        };
        // 1 with the text in ascendio_prompt_text, 0 if cancelled.
        importObject.env.ascendio_prompt = function (title, current) {
            var answer = window.prompt(get_js_object(title), get_js_object(current));
            if (answer === null) {
                return 0;
            }
            promptText = answer;
            return 1;
        };
        importObject.env.ascendio_prompt_text = function () {
            return js_object(promptText);
        };
    }

    miniquad_add_plugin({
        register_plugin: register,
        name: "ascendio_net",
        version: 1
    });
})();
