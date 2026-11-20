#[cfg(all(feature = "napi", not(target_arch = "wasm32")))]
#[cfg_attr(test, allow(dead_code))]
mod node;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
mod wasm;
