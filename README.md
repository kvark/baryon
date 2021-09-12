# baryon
[![Build Status](https://github.com/kvark/baryon/workflows/check/badge.svg)](https://github.com/kvark/baryon/actions)
[![Docs](https://docs.rs/baryon/badge.svg)](https://docs.rs/baryon)
[![Crates.io](https://img.shields.io/crates/v/baryon.svg?maxAge=2592000)](https://crates.io/crates/baryon)

Baryon is a compact 3D engine focused on fast prototyping in code.
No big dependencies, no fancy run-times, GUI editors, or other magic.
Just a simple API, based on good foundation.

Dependency highlights:
  - [wgpu](https://github.com/gfx-rs/wgpu) for GPU access
  - [winit](https://github.com/rust-windowing/winit) for windowing
  - [hecs](https://github.com/Ralith/hecs) for material ECS

Conceptually, Baryon can be seen as a descendent of [three-rs](https://github.com/three-rs/three),
pursuing similar goals, but based on a modern tech stack, and entirely disconnected from Three.js.
