'use strict';

const {sendState} = require('./index');
const {enabled, snapshot} = require('./capture');

const pending = {status: 'pending'};
const rejected = {status: 'rejected'};
const unread = Symbol('unread atom');

function attachJotaiAtom(store, atom, options) {
  if (!enabled()) return () => {};
  let current = unread;
  let revision = 0;
  let previous;
  let active = true;

  function publish(after) {
    if (previous !== undefined && !Object.is(previous, after)) {
      sendState({
        source: 'jotai',
        store: options.name,
        action: options.action ?? 'Atom changed',
        before: previous,
        after,
      });
    }
    previous = after;
  }

  function project(value, async) {
    try {
      const projected = snapshot(options.project(value));
      publish(async ? {status: 'resolved', value: projected} : projected);
    } catch {
      previous = undefined;
    }
  }

  function read() {
    let value;
    let promise;
    try {
      value = store.get(atom);
      promise = value != null && typeof value.then === 'function';
    } catch {
      value = rejected;
    }
    if (Object.is(value, current)) return;
    current = value;
    const observed = ++revision;
    if (value === rejected) {
      publish(rejected);
    } else if (promise) {
      publish(pending);
      Promise.resolve(value).then(
        result => { if (active && observed === revision) project(result, true); },
        () => { if (active && observed === revision) publish(rejected); },
      );
    } else {
      project(value, false);
    }
  }

  const unsubscribe = store.sub(atom, read);
  read();
  return () => {
    active = false;
    unsubscribe();
  };
}

module.exports = {attachJotaiAtom};
