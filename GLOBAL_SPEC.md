# space_vibe — Global Specification

space_vibe is a small 3D game prototype written in Rust using OpenGL for rendering. The player pilots a simple space shuttle through an asteroid belt. This document is the global project specification describing gameplay, controls, technical stack, architecture, and milestones.

## High-level Goals

- Build a minimal, playable prototype in Rust + OpenGL.
- Implement smooth mouse-directed steering: the shuttle points toward the mouse cursor and moves through an asteroid field.
- Keep physics and visuals simple so iteration is fast.

## Gameplay

- Player avatar: a space shuttle represented by a simple textured or colored triangle/sprite.
- Environment: continuous forward motion through an endless (or long) procedurally-generated asteroid belt.
- Objective: avoid collisions with asteroids. Optionally track score by distance/time survived.

## Controls

- Mouse position: controls shuttle direction. The shuttle's orientation always faces the mouse cursor (instant or smoothed rotation).
- Movement model: the shuttle has a forward velocity; it moves in the direction it is facing. Options:
  - Constant forward speed (simpler), or
  - Player-controlled thrust (hold a key or mouse button to accelerate) — deferred to later iteration.
- Optional: `Space` for brief boost, `R` to restart.

## Input Mapping (core requirement)

- Read mouse coordinates in window space each frame.
- Compute vector from shuttle position to mouse position; convert into world-space direction.
- Set shuttle rotation to look along that vector. Use either:
  - instant rotation: rotation = atan2(dy, dx), or
  - interpolated rotation: slerp/lerp angle for smoother turning.
- Shuttle velocity = forward_speed * direction. Apply basic damping to avoid jitter.

## Technical Stack

- Language: Rust
- Windowing & input: `winit`
- OpenGL bindings: `glow` (or `gl` + `glutin` / `glow` + `glutin`), choose based on familiarity. `glium` is an alternative higher-level option.
- Math: `nalgebra` or `cgmath` for vectors and transforms.
- Asset loading (optional): `image` for textures.
- Build: `cargo` (standard Rust toolchain).

- Game engine option: `Bevy` — a productive, data-driven Rust game engine (ECS, renderer, input, plugins). Good choice for fast prototyping of `space_vibe` if you prefer higher-level tooling over hand-rolled OpenGL.

Engine option: Bevy — recommended for faster iteration and higher-level game features (ECS, built-in renderer, plugins). Choose low-level `winit` + `glow` if your priority is learning raw OpenGL.

Example dependency suggestions for `Cargo.toml`:

```
[dependencies]
winit = "0.28"
glow = "0.11"
nalgebra = "0.32"
image = "0.24"
rand = "0.8"
```

## Architecture

- Main loop: event-driven window loop (`winit`) with a fixed timestep or variable timestep game update.
- Systems:
  - Input system: poll mouse each frame, compute desired heading.
  - Physics/motion: apply heading to shuttle position.
  - Spawner: procedural asteroid generation ahead of the shuttle; spawn with randomized positions, sizes, and velocities.
  - Collision detection: circle or AABB checks between shuttle and asteroids.
  - Renderer: render shuttle and asteroids; simple shaders, no complex lighting required.

## Art & Assets

- Initial prototype: use simple colored primitives (triangles/circles) or tiny PNG sprites.
- Keep assets minimal to focus on gameplay and engine.

## Metrics & Scoring

- Track survival time or distance traveled.
- Optionally increment score per asteroid avoided or per distance milestone.

## Milestones

1. Prototype: window + rendering + shuttle that faces the mouse and moves (core requirement).
2. Asteroid spawning and collision detection.
3. Score and UI (HUD showing time/score, restart mechanics).
4. Polish: smoothing rotation, particle effects, sounds.

## Development Notes & Next Steps

- Start by scaffolding a minimal Rust project with `winit` and OpenGL context.
- Implement mouse-to-world conversion and heading computation first; verify shuttle rotation visually.
- Add movement and simple asteroid rendering next, then collisions.
- Keep iterations small: implement one feature at a time and playtest frequently.

## Files & Organization (suggested)

- `src/main.rs` — entry and game loop
- `src/engine/` — windowing, renderer, GL helpers
- `src/game/` — shuttle, asteroid, input, spawn logic
- `assets/` — textures and optional audio

---

This document is the canonical global specification for the `space_vibe` project. Update it as the design or technical choices evolve.
