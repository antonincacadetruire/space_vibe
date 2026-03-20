# Space Vibe — TODO

Goal: Create a realistic space environment and a playable Saturn with ring asteroids.

- [x] Design starfield & background (stars + nebula)
- [x] Implement skybox / procedural nebula texture
- [x] Make sky follow the player camera
- [x] Create Saturn: textured sphere + atmosphere shader
- [x] Add asteroid ring around Saturn
- [x] Generate ring system: instanced asteroids + LOD
- [x] Spawn player on a ring asteroid and set orientation
- [x] Add physics/colliders for asteroids (e.g., bevy_rapier3d)
- [x] Polish: lighting, bloom, cinematic lighting
- [ ] Optimize further: frustum culling, GPU instancing, batching
- [ ] Add particle debris and sound FX

Status:
- Current build uses a procedural starfield, a textured Saturn sphere, and a first-pass ring belt made of asteroids.
- The remaining open work is deeper GPU instancing, frustum culling tuning, and cosmetic effects.
