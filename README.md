# space_vibe
game project for school, only vibe coding

```mermaid
graph TB
  M(Mouse Position)

  S1[Shuttle Steering]
  S2[Asteroid Spawner Timer]
  S3[Asteroid Collision]
  S4[Asteroid Movement]
  R[Renderer Camera]

  RP[ShuttlePosition resource]
  RV[VelocityUpdates resource]
  RT[AsteroidSpawnTimer]

  M --> S1
  S1 --> RP
  RT --> S2
  S2 --> Ast[Asteroid Entities]
  Ast --> S3
  S3 --> RV
  RV --> S4
  Ast --> S4
  S4 --> Ast
  RP --> S4
  S4 --> R
  S1 --> R
```