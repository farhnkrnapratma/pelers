export let native;

const offsets = (ms) => -new Date(ms).getTimezoneOffset() * 60000;

export function bind(engine) {
  native = engine;
  engine.setOffsets(offsets);
}
