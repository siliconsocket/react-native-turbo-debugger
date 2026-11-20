'use strict';

const {enabled, serialize, fitsFrame} = require('./capture');

function sendState(transition) {
  if (!enabled()) return;
  try {
    const {source, store, action, before, after, payload, correlationId} = transition;
    if (typeof source !== 'string' || !source.trim() ||
        typeof store !== 'string' || !store.trim() ||
        typeof action !== 'string' || !action.trim() ||
        (correlationId !== undefined && (typeof correlationId !== 'string' || !correlationId.trim()))) {
      return;
    }
    const json = serialize({
      source, store, action, before, after,
      ...(payload === undefined ? {} : {payload}),
      ...(correlationId === undefined ? {} : {correlationId}),
    });
    if (fitsFrame(json)) {
      require('./specs/NativeRNTurboDebuggerState').default?.enqueueState(json);
    }
  } catch {
    // Debug capture must never interfere with the application.
  }
}

module.exports = {sendState};
