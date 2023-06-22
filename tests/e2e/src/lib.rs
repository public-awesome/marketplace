#![allow(clippy::too_many_arguments)]

#[cfg(not(target_arch = "wasm32"))]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
mod helpers;
