export let native;

let zone;
let version = 0;

function offsets(ms) {
  if (Number.isNaN(ms)) {
    const current = globalThis.process?.env?.TZ;
    if (current !== zone) {
      zone = current;
      version++;
    }
    return version;
  }
  return -new Date(ms).getTimezoneOffset() * 60000;
}

export function bind(engine) {
  native = engine;
  engine.setOffsets(offsets);
}
