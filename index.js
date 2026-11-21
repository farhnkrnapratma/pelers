import { napi } from "./js/napi.js";
import { bind } from "./js/native.js";

bind(napi);

export * from "./js/core.js";
