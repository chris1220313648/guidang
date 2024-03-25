// Copyright 2020-2022 Han puyu. All rights reserved.

"use strict";

((window) => {
    const core = window.Deno.core;

    async function httpPost(url, body) {
        return await core.opAsync("op_http_post", url, body)
    }

    async function httpGet(url, body) {
        return await core.opAsync("op_http_get", url, body)
    }

    window.__bootstrap.ability = {
        httpPost,
        httpGet,
    };
})(this);