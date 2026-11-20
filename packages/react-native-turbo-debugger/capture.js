'use strict';

function enabled() {
  return typeof __DEV__ !== 'undefined' && __DEV__;
}

// Reject lossy JSON conversions, including non-finite numbers and nested undefined.
function serialize(value) {
  return JSON.stringify(value, (_, item) => {
    if (item === null || typeof item === 'object' || typeof item === 'string' ||
        typeof item === 'boolean' || Number.isFinite(item)) {
      return item;
    }
    throw new TypeError('State capture requires JSON values');
  });
}

function snapshot(value) {
  return JSON.parse(serialize(value));
}

function fitsFrame(json) {
  const limit = 16 * 1024;
  if (json.length > limit) return false;
  let bytes = 0;
  for (let i = 0; i < json.length && bytes <= limit; i++) {
    const code = json.charCodeAt(i);
    if (code <= 0x7f) {
      bytes += 1;
    } else if (code <= 0x7ff) {
      bytes += 2;
    } else {
      bytes += 3;
      if (code >= 0xd800 && code <= 0xdbff) {
        const low = json.charCodeAt(i + 1);
        if (low >= 0xdc00 && low <= 0xdfff) {
          bytes += 1;
          i++;
        }
      }
    }
  }
  return bytes <= limit;
}

module.exports = {enabled, serialize, snapshot, fitsFrame};
