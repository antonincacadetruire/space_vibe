# space_vibe
game project for school, only vibe coding

```mermaid
graph TB
  M(Mouse Position)
  K(Keyboard Input)

  SS[Procedural Space Scene Setup]
  SB[Camera-Following Starfield + Nebula Dome]
  SP[Saturn + Atmosphere]
  SRI[Asteroid Ring around Saturn]
  SR[Ring Asteroid Belt]
  RP[Rapier Physics / Colliders]
  LB[Bloom + Cinematic Lighting]
  LOD[Ring LOD System]
  ML[Mouse Look]
  PM[Player Movement]
  AC[Asteroid Collision]
  AM[Asteroid Movement]
  UI[Menu UI / Font Fallback]
  R[Main Camera / Player View]

  RV[VelocityUpdates resource]

  Startup --> SS
  SS --> SB
  SS --> SP
  SS --> SRI
  SS --> SR
  SS --> RP
  SS --> LB
  SS --> R
  R --> SB

  M --> ML
  ML --> R
  K --> PM
  PM --> R
  SR --> Ast[Asteroid Entities]
  LOD --> SR
  Ast --> AC
  AC --> RV
  RV --> AM
  Ast --> AM
  AM --> Ast
  RP --> Ast
  UI --> R
  R --> AC
  R --> AM
```