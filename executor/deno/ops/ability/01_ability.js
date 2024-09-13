// Copyright 2020-2022 Han puyu. All rights reserved.

"use strict";

((window) => {
    const core = window.Deno.core;

    async function httpPost(url, body, headers="" ) {
       

        // 传递 URL、请求体和字符串化的 headers
        return await core.opAsync("op_http_post", url, body, headers);
    }

    async function httpGet(url, body) {
        return await core.opAsync("op_http_get", url, body)
    }

    function getAbilityStatus(ability, property) {
        return core.opSync("op_get_ability_status", ability, property)
    }
    function getAbilityUrl(ability) {
        return core.opSync("op_get_ability_url", ability)
    }
    window.__bootstrap.ability = {
        httpPost,
        httpGet,
        getAbilityStatus,
        getAbilityUrl,
    };
})(this);