import init, * as engine from "./wasm/pelers.js";
import { bind } from "./js/native.js";

await init();
bind(engine);

export * from "./js/core.js";
