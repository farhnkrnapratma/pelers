import init, * as engine from "./wasm/small/pelers.js";
import { bind } from "./js/native.js";

await init();
bind(engine);

export * from "./js/core.js";
