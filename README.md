# space_vibe
game project for school, only vibe coding

```mermaid
graph TB
  M(Mouse Position)
  K(Keyboard Input)

  GS_SM[GameState: StartMenu]
  GS_PL[GameState: Playing]
  GS_DE[GameState: Dead]

  SM[Start Menu - Scene Selection + Per-Scene Best Times]
  DS[Death Screen - Score + Kill Count + PlayAgain + MainMenu]
  TI[Timer + Kill Count HUD - top-right]
  MM[Minimap - bottom-right - enemy radar]

  ActiveScene[ActiveScene Resource - SpaceAsteroids / IceCaves / DesertPlanet]
  KillCount[KillCount Resource - reset on enter Playing]
  SL[SceneLeaderboard - top-3 per scene persisted to .dat files]

  SceneMgr[spawn_active_scene_system]
  SS[Space Asteroids Scene - Saturn ring belt]
  IC[Ice Caves Scene - giant asteroid interior]
  DP[Desert Planet Scene - horizon dunes mountains]
  SceneClean[despawn_scene_entities - SceneEntity marker]

  SB[Camera-Following Starfield + Nebula Dome]
  SP[Saturn + Atmosphere]
  LOD[Distance-Based Asteroid LOD]
  ML[Mouse Look]
  PM[Player Movement]
  AC[Asteroid Collision]
  AM[Asteroid Movement]
  UI[Menu UI]
  R[Main Camera / Player View]
  BP[Bloom + Cinematic Lighting]
  AL[Alien Ships - patrol + shoot]
  MS[Homing Missiles - proportional nav]
  CB[Combat - lasers + explosions + health pips]
  LB[Leaderboard submit on death]

  Startup --> R

  GS_SM --> SM
  SM -- Scene chosen --> ActiveScene
  ActiveScene --> SceneMgr
  SM -- Play clicked --> GS_PL
  GS_PL --> SceneMgr
  SceneMgr -- SpaceAsteroids --> SS
  SceneMgr -- IceCaves --> IC
  SceneMgr -- DesertPlanet --> DP
  GS_PL --> TI
  GS_PL --> MM
  GS_PL -- collision / missile hit --> GS_DE
  GS_DE --> DS
  GS_DE --> LB
  LB --> SL
  DS -- Play Again --> GS_PL
  DS -- Main Menu --> GS_SM
  GS_PL -- exit --> SceneClean

  M --> ML
  ML --> R
  K --> PM
  PM --> R

  SS --> SB
  SS --> SP
  SS --> LOD
  IC --> BP
  DP --> BP

  AL --> CB
  MS --> CB
  CB --> KillCount
  CB -- alien destroyed + score --> LB

  MM --> AL
```

## Project Structure

```
systems/
  core/       – collision, exit, fullscreen, mouse, movement, spawner
  enemies/    – alien_ships, missiles, combat
  scenes/     – space_scene, ice_caves, desert_planet, scene_manager
  ui/         – menu, start_menu, death_screen, hud, minimap
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