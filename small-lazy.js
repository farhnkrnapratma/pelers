import init, * as engine from "./wasm/small/pelers.js";
import { lazily } from "./js/lazy.js";

export * from "./js/core.js";

export const { load, renderAsync, mountAsync } = lazily(init, engine);
