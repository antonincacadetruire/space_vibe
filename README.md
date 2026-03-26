# space_vibe
game project for school, only vibe coding

```mermaid
graph TB
  M(Mouse Position)
  K(Keyboard Input)

  GS_SM[GameState: StartMenu]
  GS_PL[GameState: Playing]
  GS_DE[GameState: Dead]

  JSON_MAPS[data/maps/*.json - MapDef: id, label, boundary_radius, SVG preview]
  JSON_SKINS[data/skins/*.json - SkinDef: id, label, SVG preview]
  JSON_ENEMIES[data/enemies/*.json - EnemyDef: colors, speed, health, spawn rules]
  JSON_LLM[data/llm_config.json - api_url, api_key, model, system_prompt]
  JSON_SECRETS[data/secrets.json - persisted API key]
  JSON_KEYS[data/keybindings.json - persistent key bindings per action]
  DataLoader[load_catalogs Startup - MapCatalog + SkinCatalog + EnemyCatalog + LlmConfig]
  SVGRast[svg_to_image - resvg rasterisation to Bevy Image handle]

  SM[Start Menu - 880px panel - Skin/Map carousels with SVG previews + best times]
  DS[Death Screen - Score + Kill Count + PlayAgain + MainMenu]
  TI[Timer + Kill Count HUD - top-right]
  MM[Minimap - bottom-right - enemy radar]
  CHAT[Copilot Chat - F2 overlay - LLM generates map/skin/enemy JSON - works in StartMenu + Playing - click blocker + auto API key prompt]
  KEYBINDS[Keybindings Resource - load/save JSON - Commands panel rebind UI]

  ActiveScene[ActiveScene Resource - SpaceAsteroids / IceCaves / DesertPlanet]
  KillCount[KillCount Resource - reset on enter Playing]
  SL[SceneLeaderboard - top-3 per scene persisted to .dat files]
  ZB[ZoneBoundary Resource - boundary_radius from MapCatalog]
  ShipSkin[ShipSkin Resource - WarPlane / Banana / Mosquito]
  CamMode[CameraMode Resource - FirstPerson / ThirdPerson toggle F5]
  CamArm[CameraArmOffset - spring-arm for ThirdPerson view]
  TerrainData[DesertTerrainData - floor_y + ellipsoid kill-zones for mountains dunes spires]

  SceneMgr[spawn_active_scene_system - reads MapCatalog for boundary]
  SS[Space Asteroids Scene - Saturn ring belt - 400k boundary]
  IC[Ice Caves Scene - giant asteroid interior - 160k boundary]
  DP[Desert Planet Scene - horizon dunes mountains rock spires - 280k boundary]
  SceneClean[despawn_scene_entities - SceneEntity marker]

  ShipModel[spawn_player_ship_system - WarPlane / Banana / Mosquito 3D mesh]
  CamView[camera_toggle_system - F5 show/hide ship model + arm offset]

  SB[Camera-Following Starfield + Nebula Dome]
  SP[Saturn + Atmosphere]
  LOD[Distance-Based Asteroid LOD]
  ML[Mouse Look]
  PM[Player Movement + zone boundary reflection 70% damping]
  AC[Asteroid Collision]
  AM[Asteroid Movement]
  TD[Terrain Death - floor + ellipsoid kill-zones]
  UI[Menu UI - Resume/Settings/Commands/Exit - Commands panel shows all bindings]
  R[Main Camera / Player View]
  BP[Bloom + Cinematic Lighting]
  AL[Alien Ships - patrol + shoot - stats from EnemyCatalog]
  MS[Homing Missiles - proportional nav]
  CB[Combat - lasers + explosions + health pips]
  LB[Leaderboard submit on death]

  Startup --> DataLoader
  JSON_MAPS --> DataLoader
  JSON_SKINS --> DataLoader
  JSON_ENEMIES --> DataLoader
  JSON_LLM --> DataLoader
  JSON_SECRETS --> DataLoader
  JSON_KEYS --> KEYBINDS
  DataLoader --> SVGRast
  SVGRast --> SM
  DataLoader --> SceneMgr
  DataLoader --> AL

  Startup --> R

  GS_SM --> SM
  GS_SM --> CHAT
  SM -- AI Chat button --> CHAT
  SM -- Map chosen --> ActiveScene
  SM -- Skin chosen --> ShipSkin
  ActiveScene --> SceneMgr
  SM -- Play clicked --> GS_PL
  GS_PL --> SceneMgr
  SceneMgr -- SpaceAsteroids --> SS
  SceneMgr -- IceCaves --> IC
  SceneMgr -- DesertPlanet --> DP
  SceneMgr --> ZB
  DP --> TerrainData
  GS_PL --> ShipModel
  GS_PL --> TI
  GS_PL --> MM
  GS_PL --> CHAT
  GS_PL -- collision / missile / terrain --> GS_DE
  GS_DE --> DS
  GS_DE --> LB
  LB --> SL
  DS -- Play Again --> GS_PL
  DS -- Main Menu --> GS_SM
  GS_PL -- exit --> SceneClean

  M --> ML
  ML --> R
  K --> KEYBINDS
  KEYBINDS --> PM
  KEYBINDS --> UI
  PM --> ZB
  ZB -- boundary bounce --> PM
  PM --> R
  TerrainData --> TD
  TD -- DeathCause::Terrain --> GS_DE

  ShipSkin --> ShipModel
  ShipModel --> CamView
  CamMode --> CamView
  CamArm --> CamView

  CHAT -- Save JSON --> JSON_MAPS
  CHAT -- Save JSON --> JSON_SKINS
  CHAT -- Save JSON --> JSON_ENEMIES

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