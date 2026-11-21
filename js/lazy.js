import { mount, registered, render } from "./core.js";
import { bind } from "./native.js";

export function lazily(init, engine) {
  let ready;
  function load(type) {
    ready ??= init().then(
      () => bind(engine),
      (error) => {
        ready = undefined;
        throw error;
      },
    );
    return ready.then(() => type === undefined || registered(type));
  }
  return {
    load,
    async renderAsync(text, options = {}) {
      await load();
      return render(text, options);
    },
    async mountAsync(element, text, options = {}) {
      await load();
      return mount(element, text, options);
    },
  };
}
