// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
// Removes the `__proto__` for security reasons.  This intentionally makes
// Deno non compliant with ECMA-262 Annex B.2.2.1
//
"use strict";
delete Object.prototype.__proto__;

((window) => {
  const core = Deno.core;
  const {
    Error,
    ObjectDefineProperty,
    ObjectDefineProperties,
    ObjectFreeze,
    Symbol,
  } = window.__bootstrap.primordials;
  const util = window.__bootstrap.util; // 06_utils.js
  const Console = window.__bootstrap.console.Console; // 02_console.js
  const internals = window.__bootstrap.internals; // 02_console.js
  const timers = window.__bootstrap.timers // timers/01_timers.js
  const devices = window.__bootstrap.devices // devices//01_device.js
  const ability = window.__bootstrap.ability // devices//01_device.js

  // https://developer.mozilla.org/en-US/docs/Web/API/WindowOrWorkerGlobalScope
  const windowOrWorkerGlobalScope = {
    console: util.nonEnumerable(
      new Console(),
    ),
    window: util.readOnly(globalThis),
    self: util.writable(globalThis),
    clearInterval: util.writable(timers.clearInterval),
    clearTimeout: util.writable(timers.clearTimeout),
    setInterval: util.writable(timers.setInterval),
    setTimeout: util.writable(timers.setTimeout),
  };

  let hasBootstrapped = false;

  function bootstrapRuntime(runtimeOptions) {
    if (hasBootstrapped) {
      throw new Error("Worker runtime already bootstrapped");
    }

    const consoleFromV8 = window.console;
    const wrapConsole = window.__bootstrap.console.wrapConsole;

    delete globalThis.bootstrap;
    hasBootstrapped = true;
    ObjectDefineProperties(globalThis, windowOrWorkerGlobalScope);

    const consoleFromDeno = globalThis.console;
    wrapConsole(consoleFromDeno, consoleFromV8);

    core.setMacrotaskCallback(timers.handleTimerMacrotask);
  
    const internalSymbol = Symbol("Deno.internal");

    const finalDenoNs = {
      core,
      internal: internalSymbol,
      [internalSymbol]: internals,
      resources: core.resources,
      close: core.close,
      memoryUsage: core.memoryUsage,
      metrics: core.metrics,
      customInspect: window.__bootstrap.console.customInspect,
      inspect: window.__bootstrap.console.inspect,
      httpPost: ability.httpPost,
      httpGet: ability.httpGet
    };
    ObjectDefineProperties(finalDenoNs, {
      noColor: util.readOnly(runtimeOptions.noColor),
    });
    ObjectDefineProperties(finalDenoNs, {
      env: util.readOnly(runtimeOptions.env),
    });
    // Remove bootstrapping data from the global scope
    delete globalThis.__bootstrap;
    // Setup `Deno` global - we're actually overriding already existing global
    // `Deno` with `Deno` namespace from "./deno.ts".
    ObjectDefineProperty(globalThis, "Deno", util.readOnly(finalDenoNs));
    ObjectDefineProperty(globalThis, "Device", util.readOnly(devices));
    ObjectFreeze(globalThis.Deno.core);
    ObjectFreeze(globalThis.Device);
  }

  ObjectDefineProperties(globalThis, {
    bootstrap: {
      value: bootstrapRuntime,
      configurable: true,
    },
  });
})(this);