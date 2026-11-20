import init, * as engine from "./wasm/pelers.js";
import { mount, registered, render } from "./js/core.js";
import { bind } from "./js/native.js";

export * from "./js/core.js";

let ready;

export function load(type) {
  ready ??= init().then(
    () => bind(engine),
    (error) => {
      ready = undefined;
      throw error;
    },
  );
  return ready.then(() => type === undefined || registered(type));
}

export async function renderAsync(text, options = {}) {
  await load();
  return render(text, options);
}

export async function mountAsync(element, text, options = {}) {
  await load();
  return mount(element, text, options);
}
