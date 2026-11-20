'use strict';

const {sendState} = require('./index');
const {enabled, snapshot} = require('./capture');

function createReduxStateMiddleware(options) {
  if (!enabled()) return () => next => action => next(action);
  return api => next => action => {
    let before;
    let type;
    try {
      type = action?.type;
      if (typeof type === 'string') before = snapshot(options.project(api.getState()));
    } catch {
      // Leave dispatch behavior to Redux and the other middleware.
    }
    const result = next(action);
    if (before !== undefined) {
      try {
        const metadata = options.describe?.(action);
        sendState({
          ...metadata,
          source: 'redux',
          store: options.name,
          action: metadata?.action ?? type,
          before,
          after: options.project(api.getState()),
        });
      } catch {
        // Preserve the result even when a projection or label callback fails.
      }
    }
    return result;
  };
}

module.exports = {createReduxStateMiddleware};
