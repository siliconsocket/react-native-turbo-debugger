'use strict';

const {sendState} = require('./index');
const {enabled} = require('./capture');

function attachZustand(store, options) {
  if (!enabled()) return () => {};
  return store.subscribe((after, before) => {
    try {
      const metadata = options.describe?.(after, before);
      sendState({
        ...metadata,
        source: 'zustand',
        store: options.name,
        action: metadata?.action ?? 'State changed',
        before: options.project(before),
        after: options.project(after),
      });
    } catch {
      // A projection or label callback must not break the subscription.
    }
  });
}

module.exports = {attachZustand};
