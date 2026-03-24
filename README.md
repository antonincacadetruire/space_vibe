# space_vibe
game project for school, only vibe coding

```mermaid
graph TB
  M(Mouse Position)
  K(Keyboard Input)

  GS_SM[GameState: StartMenu]
  GS_PL[GameState: Playing]
  GS_DE[GameState: Dead]

  SM[Start Menu - Title + Play Button]
  DS[Death Screen - Score + PlayAgain + MainMenu]
  TI[Timer UI - top-right HUD]

  SS[Procedural Space Scene Setup]
  SB[Camera-Following Starfield + Nebula Dome]
  SP[Saturn + Atmosphere]
  SRI[Asteroid Ring around Saturn]
  SR[Ring Asteroid Belt]
  BP[Asteroid Broadphase Grid - free asteroids only]
  LB[Bloom + Cinematic Lighting]
  LOD[Distance-Based Asteroid LOD - sqrt-free]
  ML[Mouse Look]
  PM[Player Movement]
  AC[Asteroid Collision - BeltAsteroids excluded]
  AM[Asteroid Movement - BeltAsteroids excluded]
  OB[Orbit + AngularVel + Collision - belt only]
  UI[Menu UI / Font Fallback]
  R[Main Camera / Player View]

  RV[VelocityUpdates resource]
  MP[Material Palette - 8 shared handles for GPU instancing]

  Startup --> SS
  SS --> SB
  SS --> SP
  SS --> SRI
  SS --> SR
  SS --> MP
  MP --> SR
  SS --> LB
  SS --> R
  R --> SB

  GS_SM --> SM
  SM -- Play clicked --> GS_PL
  GS_PL --> TI
  GS_PL -- collision detected --> GS_DE
  GS_DE --> DS
  DS -- Play Again --> GS_PL
  DS -- Main Menu --> GS_SM

  M --> ML
  ML --> R
  K --> PM
  PM --> R
  SR --> Ast[BeltAsteroid Entities - no Rapier]
  LOD --> SR
  Ast --> AC
  AC --> RV
  RV --> AM
  AM --> AM
  Ast --> OB
  OB --> Ast
  OB -- player hit --> GS_DE
  AM -- player hit --> GS_DE
  UI --> R
  R --> OB
```

## Performance Architecture

| Change | Before | After |
|---|---|---|
| GPU draw calls / frame | ~1 080 (unique material per asteroid) | ~50–100 (shared palette → auto-instanced) |
| Rapier physics bodies | 1 080 KinematicPositionBased | **0** (removed entirely) |
| Shadow map pass | All 1 080 meshes | Disabled on ring light |
| Asteroid collision system | Iterates all 1 080 every frame | Skips BeltAsteroids (no-op) |
| Asteroid movement system | Iterates all 1 080 every frame | Skips BeltAsteroids (no-op) |
| Angular velocity updates | All 1 080 / frame | Only within 50 km |
| Player swept-sphere tests | All 1 080 / frame | Pre-culled to < 3 km range |
| LOD distance check | `sqrt()` × 1 080 every 0.2 s | `distance_squared()` (no sqrt) |
| Debug build opt-level | 0 | 1 (3× faster in dev) |