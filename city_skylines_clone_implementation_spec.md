# Browser City Builder — Implementation Specification

## 0. Document Status

**Status:** Planning / implementation specification

**Primary target:** Desktop browser (mouse + keyboard)

**Game shape:** Single-player, persistent, effectively infinite city simulation

**Design goal:** Deliver the systemic depth and long-term progression of a Cities: Skylines-class city builder while making moment-to-moment play substantially simpler through rule-setting, contextual tools, procedural construction, and autonomous city evolution.

---

# 1. Product Vision

Build a browser-native city simulation in which the player is simultaneously:

- an urban planner who shapes infrastructure and land use,
- a mayor/governor who sets rules and policies,
- and, eventually, a civilization-scale steward who guides a city that can grow and transform indefinitely.

The player should **not** spend most of their time placing individual buildings. The player establishes roads, infrastructure, zones, service coverage, policies, budgets, transit networks, and strategic priorities. Buildings, parcels, businesses, neighborhoods, traffic patterns, demographic patterns, and urban form should emerge from those constraints.

The simulation must support all familiar major city-builder systems plus deeper emergent systems:

- road networks
- zoning
- traffic
- public transport
- water and sewage
- power
- garbage
- education
- healthcare
- fire and emergency services
- police / public safety
- parks and recreation
- taxation and municipal finances
- employment
- housing
- land value
- pollution
- noise
- citizen demographics
- businesses
- freight and supply chains
- construction and development
- migration
- social mobility
- neighborhood identity
- real-estate dynamics
- urban specialization
- environmental feedback
- technological progression
- political pressure / public sentiment
- disasters and long-term environmental change
- procedural city morphology
- genetic / evolutionary optimization systems
- AI/ML-assisted forecasting and adaptation

The game should be easy to understand because complexity exists primarily in the simulation, not in the number of buttons exposed to the player.

---

# 2. Core Design Principles

## 2.1 Map-first interaction

The map is the primary interface. Permanent UI should be extremely small. Most controls appear only when the player is performing a relevant task or inspecting an object.

## 2.2 Rules over micromanagement

Players make high-level decisions and infrastructure decisions. The simulation executes local behavior.

Examples:

- Player paints a residential zone; parcels are created and buildings develop organically.
- Player draws a transit corridor; stops and service demand emerge from the network.
- Player changes property tax; households and businesses respond over time.
- Player establishes an industrial district; supply chains and employment evolve around it.

## 2.3 Deterministic simulation

The simulation core must be deterministic given:

- initial world state,
- simulation version,
- player commands,
- explicit seed,
- elapsed simulation time.

Do not use uncontrolled randomness. Procedural systems may use deterministic seeded pseudo-random sequences. This enables:

- replay/debugging,
- reproducible bugs,
- save compatibility,
- deterministic procedural worlds,
- simulation tests,
- ML evaluation,
- AI planning.

## 2.4 Layered fidelity

Not every citizen, vehicle, building, or economic transaction should be simulated at maximum detail at all times.

Use multiple fidelity levels:

1. **Macro:** region / district statistics.
2. **Meso:** neighborhood / parcel / network simulation.
3. **Micro:** individual citizens, vehicles, businesses, detailed service events.
4. **Interactive micro:** objects currently visible, selected, or causally important.

The game should be able to represent millions of citizens without requiring millions of fully active simulation agents every tick.

## 2.5 Infinite-world streaming

The city exists as a procedurally addressable world made of tiles/chunks. The player can continue expanding indefinitely in practice.

World generation must depend on stable coordinates + world seed so that tiles can be regenerated deterministically instead of stored wholesale.

## 2.6 Simulation owns objective reality

Rendering, UI, ML predictions, procedural generation and future AI assistants may propose or visualize outcomes, but only the simulation authority changes objective world state.

## 2.7 ML assists; ML does not define game correctness

ML should provide useful predictions, compression, forecasting, ranking, adaptation, and optimization. It must not be required for basic simulation correctness.

If an ML model is unavailable, the game must fall back to deterministic heuristics.

## 2.8 Visual richness from geometry, shaders and procedural rules

Do not build the game around sprite sheets or imported building art.

Use:

- procedural meshes
- instancing
- shader variation
- generated façades
- generated windows
- procedural roofs
- generated vegetation
- road markings from geometry/shaders
- vector/SVG UI icons where icons are required
- procedural particles only for lightweight effects

---

# 3. Recommended Technology Stack

## 3.1 Frontend

**TypeScript + React + Vite**

React is responsible for UI/state presentation only. Do not use React as the primary renderer for the world.

## 3.2 World rendering

**Three.js using its WebGPU-capable renderer path, with WebGL2 fallback.**

Rationale:

- mature browser 3D ecosystem,
- camera / scene utilities,
- GPU instancing support,
- shaders and custom materials,
- strong fit for procedural geometry,
- lets the project avoid committing the whole application to raw WebGPU too early.

WebGPU is the preferred rendering backend because it is designed for high-performance browser graphics and GPU compute, but compatibility is not universal across all browsers. Therefore the implementation must retain a WebGL2 fallback rather than making WebGPU the only way to play. citeturn856704search1turn856704search0

## 3.3 Simulation core

**Rust**

The same Rust simulation core should be compiled for:

- native server execution,
- WebAssembly execution in the browser for client-side prediction/offline functionality.

This minimizes divergence between client and server simulation rules.

## 3.4 Server

**Rust + Axum**

Responsibilities:

- authoritative simulation,
- long-running simulation process,
- persistence,
- save/load,
- simulation snapshots,
- asynchronous jobs,
- ML service integration,
- world/chunk generation coordination.

The server should be optional for local/offline single-player operation. A packaged local deployment should be able to run the server and browser client together.

## 3.5 Persistence

**SQLite initially; PostgreSQL later only if genuinely needed.**

SQLite is sufficient for a single-player-first product. SQLite also has official WebAssembly support, which makes it viable for browser-side save/index metadata where appropriate. citeturn856704search3turn856704search6

Recommended split:

- server SQLite: authoritative saves, city state, metadata, event journal
- browser IndexedDB/OPFS: cache, local settings, local quick-save, downloaded chunks

Do not make PostgreSQL an early dependency.

## 3.6 ML runtime

Prefer **ONNX** model artifacts with platform-specific inference:

- native/server inference for larger models,
- ONNX Runtime Web for lightweight browser inference where beneficial.

ONNX Runtime Web supports WASM and GPU-oriented execution paths including WebGPU; keep lightweight models eligible for client-side execution while reserving expensive models for the server. citeturn856704search10turn856704search0

## 3.7 Tooling

Recommended repository:

```text
/apps
  /web
  /server

/crates
  /sim-core
  /world-gen
  /render-data
  /procgen
  /economy
  /transport
  /services
  /agents
  /ml-runtime
  /persistence
  /shared-protocol

/tools
  /map-generator
  /replay-runner
  /sim-benchmark
  /scenario-runner
  /ml-training

/packages
  /ui
  /protocol
```

Use a single repository with strict package ownership.

---

# 4. High-Level Runtime Architecture

```text
                           ┌───────────────────┐
                           │      Player       │
                           └─────────┬─────────┘
                                     │
                              commands / queries
                                     │
                    ┌────────────────▼────────────────┐
                    │           Browser UI             │
                    │ React + contextual controls      │
                    └────────────────┬────────────────┘
                                     │
                    ┌────────────────▼────────────────┐
                    │          Render Client           │
                    │ Three.js / WebGPU / WebGL2      │
                    └────────────────┬────────────────┘
                                     │
                         world snapshots / deltas
                                     │
                ┌────────────────────▼────────────────────┐
                │           Shared Sim Interface           │
                └────────────────────┬────────────────────┘
                                     │
            ┌────────────────────────▼────────────────────────┐
            │                 SIMULATION CORE                 │
            │                   Rust / WASM                   │
            │                                                 │
            │ Time · Climate · Roads · Parcels · Buildings   │
            │ Citizens · Firms · Traffic · Economy           │
            │ Utilities · Services · Environment             │
            │ Politics · Technology · Urban evolution        │
            └───────────────┬─────────────────┬───────────────┘
                            │                 │
                     deterministic      model inputs/outputs
                            │                 │
              ┌─────────────▼──────┐   ┌─────▼──────────────┐
              │ Procedural Systems │   │      ML Layer      │
              │ terrain            │   │ forecasts          │
              │ roads              │   │ demand             │
              │ buildings          │   │ traffic             │
              │ vegetation         │   │ valuation           │
              │ neighborhoods      │   │ anomaly detection   │
              └────────────────────┘   │ optimization        │
                                       └────────────────────┘
```

---

# 5. Simulation Time Model

Do not simulate every subsystem every rendered frame.

Use a discrete simulation clock with multiple update cadences.

Example:

```text
Render frame          30–120 Hz
UI updates             10–30 Hz
Vehicle steering       10 Hz
Pedestrian movement    5–10 Hz
Traffic flow           1–5 Hz
Citizen decisions      0.1–1 Hz
Local economy          0.1–1 Hz
Municipal finances     hourly/daily
Demography             daily/monthly
Urban development      hourly/daily
Technology             daily/monthly
Climate                hourly/daily
Long-term evolution    monthly/yearly
```

The exact rates must be data-driven and benchmarked rather than hard-coded throughout the codebase.

Use a simulation scheduler:

```rust
trait SimSystem {
    fn cadence(&self) -> SimCadence;
    fn run(&mut self, ctx: &mut SimContext);
}
```

Systems must declare their dependencies and update order.

---

# 6. Determinism and Randomness

Use an explicit deterministic RNG stream manager.

Never call a global ambient RNG from simulation code.

Use stable streams such as:

```text
WORLD_GENERATION
CLIMATE
DEVELOPMENT
ECONOMY
DEMOGRAPHICS
TRAFFIC
EVENTS
GENETIC_OPTIMIZATION
```

A procedural result should be reproducible from:

```text
world_seed + system_id + spatial_coordinates + generation_version + simulation_epoch
```

This rule is especially important for infinite terrain and procedural building generation.

---

# 7. World Representation

## 7.1 Tile/chunk model

Use a hierarchical world grid.

```text
World
 ├── Region
 │    ├── Tile
 │    │    ├── Terrain cells
 │    │    ├── Roads
 │    │    ├── Parcels
 │    │    ├── Utilities
 │    │    └── Structures
 │    └── ...
 └── ...
```

Recommended initial geometry scale:

- terrain base tile: 256 m × 256 m
- internal simulation cell: adaptive; do not require every system to use the terrain raster
- region: 16 × 16 tiles or equivalent

Do not expose these dimensions as player-facing constraints.

## 7.2 Sparse world state

Store only meaningful deviations from procedural defaults.

A newly discovered tile should be generated from the world seed. Once modified by the player or persistent simulation, the changes are serialized as a delta.

```text
Procedural base(tile_seed)
          +
Persistent delta
          =
Current tile
```

## 7.3 Coordinate system

Use 64-bit integer world coordinates for simulation-space positions and chunk identifiers.

Convert to floating-point local coordinates only for rendering.

This avoids precision problems as the city expands over very large distances.

---

# 8. Terrain and Environment Generation

Terrain should be generated in layers.

```text
Seed
 ↓
continental structure
 ↓
height field
 ↓
geology / soil
 ↓
river network
 ↓
watersheds
 ↓
coastline / lakes
 ↓
climate field
 ↓
vegetation suitability
 ↓
resource distribution
 ↓
initial ecological state
```

Use combinations of:

- noise fields,
- domain warping,
- erosion approximations,
- watershed algorithms,
- Voronoi regions,
- deterministic graph generation,
- cellular automata where appropriate.

Do not run expensive erosion or watershed computation every time a tile is viewed. Generate asynchronously and cache results.

---

# 9. Procedural Road Generation

Roads are one of the game's most important procedural structures.

Represent a road network as a graph:

```text
RoadGraph
  Node
    world_position
    junction_type
    traffic_signals

  Edge
    start
    end
    lanes
    speed_limit
    allowed_modes
    width
    grade
    surface
    district
```

Player road tool:

1. Click to place first point.
2. Drag to preview.
3. Release to commit.
4. Tool automatically:
   - snaps to nearby roads,
   - computes junctions,
   - inserts required graph nodes,
   - adjusts curvature,
   - creates road mesh,
   - generates sidewalks/markings,
   - updates navigation graph.

Allow:

- straight roads,
- curved roads,
- multi-segment roads,
- bridges,
- tunnels,
- ramps,
- pedestrian paths,
- cycle paths,
- transit-only corridors,
- highways.

Later procedural systems may recommend roads, but the player retains authority over player-created infrastructure.

---

# 10. Zoning and Parcel Generation

Zoning should not directly spawn buildings.

Pipeline:

```text
Road graph
    ↓
Buildable frontage
    ↓
Parcel subdivision
    ↓
Zoning rules
    ↓
Development demand
    ↓
Building archetype selection
    ↓
Procedural building generation
    ↓
Occupancy / business assignment
```

Zones should include at minimum:

- low-density residential
- medium-density residential
- high-density residential
- mixed-use
- commercial
- office
- light industry
- heavy industry
- logistics/freight
- special-use / institutional
- parks/open space
- future development

Do not hard-code a small finite list of visually unique building assets.

Instead define building archetypes from parameters:

```text
height
floor_count
footprint
setback
roof_type
facade_type
window_density
balcony_probability
commercial_frontage
parking_requirement
construction_era
wealth_level
local_materials
energy_standard
```

---

# 11. Procedural Building System

Use procedural building grammars inspired by L-systems and shape grammars.

Example abstraction:

```text
BUILDING
 ├── BASE
 ├── BODY
 │    ├── FLOOR
 │    │    ├── WINDOW_BAY
 │    │    └── WALL_BAY
 │    └── ...
 └── ROOF
```

Parameters should determine grammar expansion.

The generator should produce:

- low-rise houses,
- apartments,
- towers,
- row houses,
- offices,
- shops,
- warehouses,
- factories,
- schools,
- hospitals,
- civic buildings,
- stations,
- parks,
- specialized structures.

The visual generator must be deterministic.

## 11.1 Architectural evolution

Building styles should evolve from simulation state rather than being purely cosmetic.

Inputs:

- local wealth
- local materials
- construction technology
- climate
- land value
- zoning regulation
- historical period
- energy rules
- culture
- population density

This lets the city develop recognizable districts without requiring hand-authored sprite sets.

---

# 12. L-System Role

Use L-systems for structural systems, not as a universal hammer.

Strong uses:

- street trees
- vegetation
- hedges
- branching road-like structures
- pedestrian green networks
- riverbank vegetation
- facade ornamentation
- certain building families
- utility branching where applicable

For roads, use graph and spline algorithms first; L-systems can generate candidate structures but should not replace proper road topology.

---

# 13. City Systems

Implement the following as independent but communicating simulation systems.

## 13.1 Population

Track:

- age
- household
- income
- education
- employment
- commute
- health
- wealth
- housing
- satisfaction
- mobility
- migration pressure
- household composition

Do not update every individual with a full decision tree each tick.

Use hierarchical cohorts, then promote selected individuals to micro-agents when necessary.

## 13.2 Households

Households select housing and locations using utility functions based on:

- price
- commute
- neighborhood quality
- access to services
- school quality
- pollution
- safety
- social ties
- amenities
- prestige

A household can:

- remain,
- move locally,
- move elsewhere in the city,
- migrate into the city,
- leave the city.

## 13.3 Businesses

Businesses have:

- demand
- employees
- wages
- input dependencies
- output production
- operating costs
- rent
- taxes
- market access
- logistics requirements
- bankruptcy pressure
- expansion/contraction state.

Allow firms to emerge through economic conditions rather than only through player placement.

## 13.4 Employment

Jobs connect households to businesses and institutions.

Model:

```text
Worker skills
       ↕
Job requirements
       ↕
Wages
       ↕
Commute cost
       ↕
Location
```

This creates meaningful reasons for different land uses to cluster.

---

# 14. Economy and Supply Chains

The economic model should go beyond a simple money-per-building system.

Represent major flows:

```text
Resources
  ↓
Extraction
  ↓
Processing
  ↓
Manufacturing
  ↓
Distribution
  ↓
Retail / services
  ↓
Households
```

Use aggregated commodity markets initially.

Each commodity can have:

- production,
- inventory,
- price,
- demand,
- transport requirement,
- substitution class.

Later allow individual firms to create localized deviations from equilibrium.

---

# 15. Traffic Simulation

Traffic must be treated as both a physical network problem and an economic/social problem.

Use a hybrid approach:

1. network-level traffic assignment for large populations,
2. explicit vehicle agents for visible / causally important traffic,
3. microscopic simulation near intersections and selected corridors.

Route choice should depend on:

- travel time,
- congestion,
- tolls,
- transfers,
- reliability,
- preference,
- vehicle type.

Use cached route graphs and incremental recomputation when road topology changes.

## 15.1 Traffic visualization

The normal map should remain readable.

When the Traffic lens is activated:

- roads become heat-mapped by congestion,
- moving vehicle density becomes visible,
- bottlenecks display compact indicators,
- selected roads show throughput and average delay.

Never require a separate traffic window to understand a major problem.

---

# 16. Public Transport

Support a unified network abstraction.

Modes:

- bus
- tram
- metro/subway
- train
- ferry
- pedestrian
- bicycle
- future high-capacity modes.

Player interaction:

1. Choose mode.
2. Draw corridor.
3. Select or auto-create stops.
4. Set service level.
5. Optionally assign a route name.

The game should automatically estimate:

- likely ridership,
- vehicle count,
- operating cost,
- service coverage,
- transfer demand.

Advanced players can tune details, but defaults must be sensible.

---

# 17. Utilities

Model:

- electricity generation/distribution
- water supply
- sewage
- waste treatment
- district heating/cooling where applicable
- communications infrastructure as a later deep system.

Use network graphs with flow calculations.

The player mostly builds networks and adjusts capacities rather than managing every node.

Example contextual indicator:

```text
Water system
Capacity: 92%
Demand trend: ↑
Reserve: 11%

[Expand network]
```

---

# 18. Public Services

Implement service systems as coverage + capacity + response-time networks.

Examples:

- healthcare
- fire
- police
- education
- parks
- sanitation
- emergency response
- transit.

A service building should affect a dynamic service field, not simply flip a radius-based boolean.

Service effectiveness depends on:

- capacity,
- staff,
- travel time,
- demand,
- quality,
- funding,
- congestion,
- adjacent network conditions.

---

# 19. Environment and Climate

Model:

- temperature
- rainfall
- groundwater
- river flow
- air pollution
- noise pollution
- soil quality
- vegetation
- heat-island effect
- flooding
- fire risk
- ecosystem health.

Environmental state should feed back into:

- health,
- migration,
- land value,
- agriculture,
- construction,
- infrastructure degradation,
- insurance/economic pressure,
- city policy.

---

# 20. City Policies and Governance

Policies should be high-leverage controls.

Examples:

- property tax rates
- zoning restrictions
- density bonuses
- parking rules
- transit subsidies
- industrial emissions limits
- building energy standards
- road pricing
- pedestrianization
- school funding
- healthcare funding
- green incentives
- development grants
- business incentives
- public housing.

Policies should produce indirect consequences rather than simple +/- modifiers.

---

# 21. Politics and Public Sentiment

Implement a lightweight but deep political layer.

Track population attitudes toward:

- housing prices
- taxes
- traffic
- safety
- pollution
- growth
- development style
- services
- public debt
- neighborhood change.

Citizens need not form an explicit political party system immediately.

Start with:

```text
Issue
 ↓
Population groups
 ↓
Sentiment
 ↓
Public pressure
 ↓
Policy consequences
```

Later add:

- elections,
- factions,
- lobbying,
- protests,
- campaigns,
- coalition formation.

---

# 22. City Evolution

The city should not be frozen in the player's initial style.

District identity emerges from historical conditions.

Each neighborhood tracks a historical profile:

```text
origin period
founding cause
dominant industries
architectural era
wealth history
demographic history
transport history
cultural traits
land-use history
```

This gives neighborhoods persistence and recognizable character.

---

# 23. Technology and Long-Term Progression

Progression should eventually become open-ended.

Instead of a simple finite tech tree, model technology as a graph of capabilities.

```text
Technology
 ├── prerequisites
 ├── applications
 ├── cost
 ├── adoption
 ├── diffusion rate
 └── obsolete alternatives
```

New technology can alter:

- building construction,
- transit,
- utilities,
- industry,
- agriculture,
- environmental management,
- governance,
- logistics.

A sufficiently old city can therefore look fundamentally different from a newly founded city.

---

# 24. Infinite Progression Model

The game should never present a hard “you won” state.

Progression continues through:

```text
Settlement
 ↓
town
 ↓
small city
 ↓
regional city
 ↓
metropolis
 ↓
megacity
 ↓
polycentric urban region
 ↓
large-scale urban civilization
 ↓
new technological / environmental eras
```

Progression metrics should include more than population:

- economic complexity
- infrastructure capacity
- technological maturity
- environmental resilience
- education
- quality of life
- network centrality
- cultural diversity
- regional influence
- institutional complexity.

A player can therefore pursue different forms of success.

---

# 25. AI/ML Architecture

AI/ML should be divided by the problem it solves.

## 25.1 Do not use ML for deterministic facts

Do not use ML to decide:

- whether a pipe is physically connected,
- whether a building is inside a parcel,
- whether a road intersects another road,
- whether electricity can physically flow,
- exact game rules.

Those remain algorithmic.

## 25.2 Appropriate ML uses

Use ML for:

- traffic demand prediction,
- development demand forecasting,
- property valuation,
- business failure prediction,
- migration forecasting,
- service demand forecasting,
- congestion forecasting,
- infrastructure failure prediction,
- land-use recommendation,
- player-facing “what happens if?” estimates,
- anomaly detection,
- behavioral clustering,
- model-based surrogate functions for expensive simulation.

## 25.3 Baseline-first requirement

Every ML subsystem must have a deterministic baseline.

```text
Prediction request
       ↓
ML model available?
  /              \
 yes              no
  ↓                ↓
ML prediction   heuristic
  \                /
   └──── validation
```

If ML performance is worse than the baseline, disable it.

## 25.4 Training architecture

Do not train models inside the live simulation by default.

Training pipeline:

```text
Simulations
   ↓
Scenario dataset
   ↓
Feature extraction
   ↓
Offline training
   ↓
Validation
   ↓
ONNX export
   ↓
Versioned model registry
   ↓
Runtime inference
```

Evolutionary algorithms may be used to tune policies, procedural parameters, network structures, or scenario configurations.

---

# 26. Genetic Algorithms

Use genetic algorithms where the search space is naturally combinatorial.

Good candidates:

- road layout proposals,
- transit route configurations,
- district layouts,
- utility network expansion plans,
- policy portfolios,
- zoning mixes,
- procedural building parameters.

Bad candidates:

- real-time vehicle movement,
- simple arithmetic,
- direct deterministic graph operations.

Use a multi-objective fitness function.

Example:

```text
fitness =
  0.25 * commute_quality
+ 0.20 * economic_output
+ 0.15 * service_access
+ 0.15 * environmental_health
+ 0.10 * fiscal_health
+ 0.10 * housing_affordability
+ 0.05 * resilience
```

Fitness weights should be exposed to the policy layer and eventually influenced by player priorities.

The genetic system should normally generate **recommendations**, not secretly alter the city.

---

# 27. Procedural Urban Growth

City growth should have several interacting generators:

```text
Population demand
Business demand
Infrastructure capacity
Terrain constraints
Land value
Accessibility
Planning rules
Historical morphology
Environmental constraints
        ↓
Development pressure field
        ↓
Candidate parcels
        ↓
Building / land-use selection
        ↓
Construction
        ↓
Neighborhood feedback
```

This is where procedural generation, ML forecasting, and deterministic simulation should meet.

---

# 28. Rendering Architecture

## 28.1 Do not create one Three.js object per building

Use:

- instanced meshes,
- geometry batching,
- GPU buffers,
- texture atlases only where genuinely useful,
- procedural shader parameters.

The city renderer should organize entities into render batches by:

```text
material family
geometry family
LOD
visibility state
chunk
```

## 28.2 Level of detail

Use at least four visual levels:

### LOD0 — overview

- broad building masses
- simplified roads
- minimal vegetation

### LOD1 — city view

- building shapes
- major windows/facade bands
- road details

### LOD2 — neighborhood

- richer facades
- trees
- street furniture
- vehicles

### LOD3 — close inspection

- selected building detail
- service activity
- pedestrians
- local effects.

Only LOD3 should be expensive.

## 28.3 Culling

Culling must happen by chunk first, then render batch.

The renderer must never iterate through every city object each frame.

---

# 29. Procedural Visual Style

Recommended visual language:

- clean semi-realistic 2.5D/3D
- orthographic or shallow-perspective default camera
- simple materials
- restrained color palette
- high readability
- strong silhouettes
- soft ambient shading
- limited post-processing.

The objective is a city that becomes visually rich through density and variation, not through expensive assets.

---

# 30. Camera and Navigation

Primary controls:

- middle mouse drag: pan
- WASD / arrow keys: pan
- mouse wheel: zoom
- right mouse drag: rotate
- Q/E: rotate
- Home: frame city
- F: focus selected object
- Shift + wheel: fast zoom

Camera should have smooth interpolation but remain responsive.

Use an orthographic or low-perspective city camera by default.

At high zoom, transition toward a perspective view if required for visual depth.

---

# 31. Minimal UI Specification

The permanent interface should contain only:

```text
┌──────────────────────────────────────────────────────────────┐
│ Population  Money  City Health  Traffic  Date/Speed  Pause │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│                                                              │
│                         CITY WORLD                            │
│                                                              │
│                                                              │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ Build   Zone   Services   Transit   Utilities   Policies   🔎│
└──────────────────────────────────────────────────────────────┘
```

Do not add permanent mini-panels for every subsystem.

---

# 32. Universal Command Ribbon

The bottom ribbon is the primary action surface.

Categories:

```text
BUILD
ZONE
SERVICES
TRANSIT
UTILITIES
POLICIES
```

Clicking a category opens a lightweight contextual strip.

Example:

```text
BUILD
────────────────────────────────────────────────────────
Road   Highway   Path   Bridge   Tunnel   District tool
```

The ribbon must close automatically after a command is completed unless the user pins it.

---

# 33. Contextual Object Inspector

Clicking an object opens a compact inspector anchored near the selected object.

Example:

```text
┌─────────────────────────┐
│ Riverside Apartments    │
│ Residential · 214 units │
├─────────────────────────┤
│ Occupancy       97%     │
│ Avg. rent       ₹1,240  │
│ Land value      High    │
│ Condition       Good    │
│ Energy          82%     │
├─────────────────────────┤
│ [Details] [Upgrade]     │
└─────────────────────────┘
```

The inspector should have progressively deeper information rather than showing everything immediately.

---

# 34. Universal Lens System

Every important invisible system should be inspectable through the same lens control.

Lenses:

- Normal
- Traffic
- Land Value
- Population
- Wealth
- Employment
- Education
- Healthcare
- Pollution
- Noise
- Water
- Electricity
- Waste
- Crime
- Fire Risk
- Transit
- Development Pressure
- Environment
- Housing
- Industrial Supply.

Lens interaction should be one click / hotkey, not a new management screen.

A lens is a rendering mode over the same world, not a separate map.

---

# 35. Information Hierarchy

The game should always answer:

1. **What is happening?**
2. **Why is it happening?**
3. **What can I do about it?**
4. **What will likely happen if I do that?**

Example:

```text
Traffic congestion ↑

WHY?
North district employment +18%
Road capacity +2%

TRY
Add transit
Expand arterial
Move freight route

FORECAST
-12% congestion
+₹420/day operating cost
```

The last section can be powered by deterministic analysis first and ML forecasts later.

---

# 36. Easy Player Interaction

Every major action should follow this pattern:

```text
Select tool
 → preview
 → drag/click
 → automatic validation
 → commit
 → simulation responds
```

Avoid modal configuration dialogs during ordinary construction.

Advanced configuration should be available through a secondary “Details” action.

---

# 37. Smart Defaults

Players should rarely have to specify low-level values.

When drawing a road, automatically infer:

- reasonable road width,
- lane configuration,
- sidewalks,
- markings,
- speed limit,
- intersection behavior.

When creating a bus route, automatically infer:

- stop placement,
- vehicle count,
- schedule,
- depot relationship.

When zoning an area, automatically infer:

- parcel sizes,
- setbacks,
- development density,
- building selection.

Advanced users can override these defaults.

---

# 38. Notifications

Do not create a constant stream of popups.

Use a small event queue.

```text
⚠ Water system near capacity
⚠ North district congestion
● New university graduates entering workforce
▲ Industrial demand rising
```

Notifications should be ranked by:

```text
severity × player relevance × expected impact
```

Allow the simulation to merge repetitive notifications.

---

# 39. Time Controls

Provide simple controls:

```text
▶ 1×  2×  5×  10×  25×  ⏸
```

Later allow custom simulation speed.

When the player is inspecting a critical issue, optionally reduce time speed automatically and provide a clear indicator.

Do not silently pause without telling the player.

---

# 40. Save and Persistence

The city must persist indefinitely.

Use:

```text
World metadata
Simulation epoch
Seed
Simulation version
City state
Modified chunks
Historical summaries
Event journal
Player settings
Model versions
```

## 40.1 Save strategy

Use incremental snapshots plus event/journal data.

Do not rewrite the entire world on every save.

Recommended:

```text
city.db
  metadata
  global_state
  chunks
  agents
  networks
  historical_metrics
  event_journal
  save_versions
```

## 40.2 Offline progression

For offline browser sessions, advance time using a bounded catch-up algorithm rather than simulating every missing frame.

Example:

```text
player leaves at T0
player returns at T1
        ↓
calculate elapsed time
        ↓
macro simulation
        ↓
medium simulation
        ↓
reconcile pending micro-events
```

Never blindly multiply a frame loop by elapsed time.

---

# 41. Server / Browser Responsibility

## Browser

Responsible for:

- rendering,
- input,
- camera,
- UI,
- local cache,
- lightweight visual simulations,
- optional lightweight ML inference,
- prediction/interpolation.

## Server / native sim runtime

Responsible for:

- authoritative simulation,
- long-running progression,
- persistence,
- expensive AI/ML,
- world generation jobs,
- large-scale routing,
- background simulation.

For local single-player, the server can be bundled as a local process.

---

# 42. Browser Connectivity Model

The web client should maintain a local state machine:

```text
CONNECTED
SYNCING
OFFLINE
RECONNECTING
```

If disconnected:

- continue rendering the cached city,
- allow safe local actions where possible,
- queue commands,
- mark unsynced changes clearly,
- reconcile against authoritative state after reconnect.

Never silently discard player commands.

---

# 43. Networking Model

Do not stream the whole city every tick.

Use:

```text
initial snapshot
+
chunk requests
+
state deltas
+
event messages
+
periodic checkpoints
```

The browser should subscribe to regions around the camera plus strategically important simulation objects.

---

# 44. Data-Oriented Simulation

Avoid an architecture with thousands of deeply nested object references.

Prefer component-oriented storage for hot simulation paths.

Example:

```text
Citizen IDs
  → age array
  → income array
  → education array
  → household ID array
  → employment ID array
```

Use stable IDs for relationships.

Cold data may remain structured.

---

# 45. Event System

Cross-system communication should use explicit events.

Examples:

```text
RoadBuilt
ParcelCreated
BuildingStarted
BuildingCompleted
BusinessOpened
BusinessClosed
CitizenMoved
JobCreated
TransitRouteChanged
PowerShortage
WaterShortage
FloodStarted
PolicyChanged
TaxChanged
TechnologyDiscovered
```

Use event sourcing selectively for history/debugging, not as an excuse to make every state mutation an event.

---

# 46. Historical Data

The city should remember its history.

Store compressed historical metrics:

- population
- GDP/economic output
- employment
- traffic
- land values
- housing prices
- pollution
- education
- public debt
- service coverage.

At long time scales, downsample older records.

Example:

```text
last 7 days      hourly
last 2 years     daily
last 20 years    monthly
all earlier      yearly
```

This enables city-history visualizations without unbounded memory growth.

---

# 47. Testing Philosophy

The project must use an evidence-first testing methodology.

Every implementation stage should follow:

```text
inspect
 → narrow hypothesis
 → smallest change
 → syntax/type validation
 → unit tests
 → system tests
 → real gameplay path
 → investigate anomalies
 → benchmark
```

Do not “fix” a failed test by weakening the test before determining why it failed.

---

# 48. Simulation Invariants

Create automated invariant tests such as:

- population cannot become negative,
- money cannot become NaN,
- road graphs remain valid,
- parcels do not overlap illegally,
- water mass remains conserved within modeled tolerances,
- utility networks do not contain invalid links,
- citizens cannot have impossible employment references,
- buildings cannot exist outside valid parcels unless explicitly tagged as infrastructure,
- time is monotonic,
- deterministic replay produces identical state hashes.

Every major subsystem should publish invariants.

---

# 49. Deterministic Replay System

Implement a replay/debug mechanism.

A replay consists of:

```text
seed
simulation version
initial state
player command stream
model versions
```

Running the replay should reproduce the same major state hash.

This is essential for diagnosing emergent system bugs.

---

# 50. Scenario Runner

Create a headless simulator executable:

```bash
citysim scenario scenario_name --years 100
```

It should be able to run without the browser.

Use it for:

- regression tests,
- balance testing,
- ML dataset generation,
- genetic algorithm evaluation,
- performance benchmarks.

---

# 51. Performance Targets

Initial targets, subject to benchmarking:

### Browser

- 60 FPS target at normal city zoom.
- 30 FPS minimum during extreme city density on supported hardware.
- no full-city object iteration per frame.
- render frame should be decoupled from simulation tick.

### Simulation

The simulation should support:

- at least hundreds of thousands of citizens represented meaningfully,
- millions of conceptual citizens through hierarchical aggregation,
- large road networks,
- thousands to tens of thousands of buildings,
- long-running simulation without unbounded memory growth.

### Memory

Do not allocate large transient objects inside hot loops.

Benchmark allocations continuously.

---

# 52. Debug / Developer UI

Developer mode should expose:

```text
FPS
frame time
sim tick time
network latency
visible chunks
visible objects
active agents
simulation backlog
memory usage
GPU renderer
ML inference time
world seed
state hash
```

Add toggles for:

- road graph
- parcel graph
- utility graphs
- navigation graph
- traffic flow
- population density
- development pressure
- service fields
- procedural generation boundaries.

This UI must not ship as part of the ordinary player interface.

---

# 53. Accessibility and Usability

Even though the primary target is desktop, maintain:

- keyboard navigation for UI,
- high-contrast lens modes,
- scalable UI text,
- reduced-motion mode,
- tooltips explaining unfamiliar concepts,
- reversible actions where practical,
- clear undo availability.

---

# 54. Undo / Command Model

Player construction actions should be command objects.

```rust
trait PlayerCommand {
    fn validate(&self, world: &World) -> ValidationResult;
    fn apply(&self, world: &mut World);
    fn inverse(&self) -> PlayerCommand;
}
```

This enables:

- undo,
- replay,
- networking,
- deterministic tests,
- command history.

Do not mutate world state directly from UI event handlers.

---

# 55. Player Feedback Before Commit

Every player action needs a preview state.

For road placement:

```text
valid → normal preview
invalid → clearly visible invalid preview
warning → valid but problematic
```

Example warning:

```text
⚠ This road may create a severe congestion bottleneck.
```

Do not block experimentation unless an action is physically impossible.

---

# 56. Recommendation Layer

The game may recommend actions without taking them.

Example:

```text
North District

Housing shortage detected.

Recommended:
+ medium-density zoning
+ transit corridor
+ school expansion

Expected impact:
Housing pressure ↓
Transit demand ↑
Tax base ↑
```

Recommendations can use heuristics, simulation forecasts, ML and eventually genetic optimization.

The player remains the decision-maker.

---

# 57. Procedural Content Registry

Avoid scattering generation rules throughout code.

Define versioned registries:

```text
BuildingArchetypeRegistry
RoadArchetypeRegistry
VegetationRegistry
DistrictStyleRegistry
PolicyRegistry
TechnologyRegistry
ServiceRegistry
CommodityRegistry
```

All registries should be serializable and versioned.

---

# 58. Modularity

Systems must not depend on implementation details of unrelated systems.

For example:

Bad:

```text
TrafficSystem directly modifies Citizen.money
```

Good:

```text
TrafficSystem emits CommuteCostChanged
Economy/Demography systems consume it
```

Use explicit interfaces and data contracts.

---

# 59. First Play Experience

The first 20 minutes should teach the game without tutorials that cover the whole system.

Start with:

1. Choose a procedurally generated starting area.
2. Draw a road.
3. Paint residential/commercial/industrial zones.
4. Build basic water and power.
5. Watch development emerge.
6. Observe traffic.
7. Build a service.
8. Adjust one policy.
9. Expand.

Advanced systems should reveal themselves as the city grows.

Do not expose the entire policy/economy/technology interface at the beginning.

---

# 60. Progression of Complexity

Use **system discovery**, not artificial complexity.

Example:

```text
Start
 ├─ Roads
 ├─ Zones
 ├─ Water
 └─ Power

Growth
 ├─ Services
 ├─ Education
 ├─ Traffic
 └─ Transit

Maturity
 ├─ Economy
 ├─ Industry
 ├─ Housing markets
 ├─ Politics
 └─ Environment

Advanced
 ├─ Technology
 ├─ Regional economy
 ├─ Genetic planning
 ├─ Advanced logistics
 └─ Long-term urban evolution
```

The simulation may support systems before they are made visible to the player.

---

# 61. Build Order

Do not attempt the full simulation at once.

## Phase 0 — Technical skeleton

Implement:

- repository
- Rust simulation crate
- web frontend
- Three.js renderer
- WebGPU capability detection
- WebGL2 fallback
- camera
- chunk system
- command protocol
- save/load skeleton
- deterministic RNG
- fixed simulation clock.

Deliverable: empty procedural world that can be navigated and saved.

## Phase 1 — Terrain + roads

Implement:

- terrain generator
- water
- procedural vegetation
- road graph
- road drawing tool
- procedural road rendering
- chunk streaming.

Deliverable: player can draw a road network over an infinite procedural landscape.

## Phase 2 — Zoning + procedural development

Implement:

- parcel generation
- zoning
- development demand
- building archetypes
- procedural buildings
- occupancy.

Deliverable: a city can grow itself from player-created roads and zones.

## Phase 3 — Core population + economy

Implement:

- households
- jobs
- businesses
- housing
- taxation
- income
- basic migration.

Deliverable: the city behaves like an actual economy rather than a visual construction toy.

## Phase 4 — Utilities + services

Implement:

- power
- water
- sewage
- waste
- healthcare
- education
- police
- fire
- parks.

Deliverable: basic municipal management.

## Phase 5 — Traffic + transit

Implement:

- navigation graph
- route choice
- traffic assignment
- explicit vehicles around player
- buses
- pedestrian network
- transit UI.

Deliverable: a traffic-driven city.

## Phase 6 — Environment + deeper economy

Implement:

- pollution
- land value
- supply chains
- logistics
- environmental feedback
- resource flows.

Deliverable: city systems become mutually dependent.

## Phase 7 — Governance + city history

Implement:

- policy system
- public sentiment
- district identity
- historical metrics
- long-term urban evolution.

Deliverable: cities acquire persistent identity.

## Phase 8 — AI/ML layer

Implement:

- demand prediction
- traffic forecasting
- service demand forecasting
- property valuation
- anomaly detection
- recommendation engine.

Only train models after enough simulation data exists.

## Phase 9 — Genetic optimization

Implement:

- road-layout candidates
- transit optimization
- zoning recommendations
- policy optimization.

Make all results optional recommendations first.

## Phase 10 — Technology and open-ended progression

Implement:

- technology graph
- changing building archetypes
- evolving infrastructure
- regional interaction
- advanced environmental systems.

---

# 62. OpenCode Agent Development Protocol

The project will be built by coding agents. The agents must follow strict boundaries.

## 62.1 Before changing code

Every agent must:

1. inspect the current architecture,
2. identify owners of the relevant functionality,
3. identify existing tests,
4. identify serialization/protocol contracts,
5. formulate a narrow implementation hypothesis.

## 62.2 Change policy

Prefer the smallest coherent change.

Do not rewrite existing subsystems merely because a different design is personally preferred.

Do not introduce a dependency without validating:

- license,
- bundle/runtime impact,
- browser support,
- maintenance status,
- actual need.

## 62.3 Verification order

```text
format
↓
type check / compile
↓
unit tests
↓
system tests
↓
scenario runner
↓
real browser gameplay path
↓
performance benchmark
```

## 62.4 Agent responsibilities

Agents should operate in narrow domains.

Suggested roles:

```text
architecture agent
simulation agent
rendering agent
procedural generation agent
traffic agent
economy agent
UI/interaction agent
ML agent
QA agent
performance agent
```

Agents must not silently redesign shared contracts.

## 62.5 Shared contract changes

Changes to these require explicit review/tests:

- simulation state schema
- command protocol
- entity IDs
- save format
- chunk serialization
- renderer data contract
- model interface.

---

# 63. Agent Task Format

Every OpenCode implementation task should have this structure:

```text
TASK
Goal:

Scope:

Non-goals:

Existing contracts:

Implementation requirements:

Tests required:

Performance requirements:

Acceptance criteria:
```

Agents should return:

```text
Implemented
Changed files
Tests run
Observed results
Known limitations
Regression risks
```

---

# 64. Anti-Patterns

The agents must not:

- couple UI directly to simulation mutation,
- put business logic into React components,
- make renderer objects authoritative state,
- simulate every citizen every frame,
- create one scene object per entity without batching,
- use arbitrary randomness,
- store the entire infinite world eagerly,
- rely on ML for deterministic game rules,
- add third-party assets merely to make the city look richer,
- create a separate management screen for every subsystem,
- add systems before the foundational simulation contracts are stable.

---

# 65. Acceptance Criteria for the Core Game

The first serious milestone should satisfy all of the following:

### Player

- Can create roads with mouse gestures.
- Can zone land without manually placing buildings.
- Can see procedural development occur.
- Can inspect any important object.
- Can toggle simulation lenses.
- Can change time speed.
- Can undo major construction actions.
- Can save and reload the city.

### Simulation

- Population grows and migrates.
- Businesses open/close.
- Jobs connect to citizens.
- Housing demand changes.
- Utilities flow through networks.
- Services influence outcomes.
- Traffic responds to infrastructure.
- City form evolves.

### World

- New land can be generated indefinitely.
- Generated terrain is deterministic.
- Modified chunks persist.
- Camera can travel far from the city without breaking numerical precision.

### Rendering

- No external building sprite library is required.
- Buildings are procedurally generated.
- Rendering uses batching/instancing.
- LOD is functional.
- WebGPU is used when available, with WebGL2 fallback.

### Engineering

- Simulation is deterministic.
- Headless scenario tests exist.
- Replay works for at least a small city.
- Save format is versioned.
- Simulation state has validation/invariant checks.

---

# 66. Suggested Initial UI Mockup

The target visual hierarchy is:

```text
┌──────────────────────────────────────────────────────────────┐
│ 12,482 👥   ₹2.4M   😊 82   🚗 71%   2037-05-14   ▶ 2×     │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│                                                              │
│                      PROCEDURAL CITY                         │
│                                                              │
│                ╭─────╮                                       │
│        ════════╪═════╪════════════                            │
│              ╱ │     │ ╲                                     │
│             ╱  │     │  ╲                                    │
│                                                              │
│                                                              │
│                              ┌──────────────────────┐        │
│                              │ Northside Apartments │        │
│                              │ 214 units · 97%      │        │
│                              │ Land value: High     │        │
│                              │ Traffic access: Good │        │
│                              └──────────────────────┘        │
├──────────────────────────────────────────────────────────────┤
│ BUILD │ ZONE │ SERVICES │ TRANSIT │ UTILITIES │ POLICIES │ 🔍│
└──────────────────────────────────────────────────────────────┘
```

The UI should feel closer to an instrument panel than a conventional strategy-game window jungle.

---

# 67. Key Interaction Examples

## Building a neighborhood

```text
ZONE → Residential
      ↓
paint region
      ↓
preview parcels
      ↓
commit
      ↓
development pressure increases
      ↓
parcels get developed
      ↓
buildings generated procedurally
      ↓
households move in
```

## Fixing congestion

```text
Traffic lens
      ↓
select bottleneck
      ↓
inspect cause
      ↓
recommendations appear
      ↓
player chooses transit / road / zoning intervention
      ↓
simulation updates
      ↓
forecast displays expected effect
```

## Creating transit

```text
TRANSIT → BUS
      ↓
draw corridor
      ↓
automatic stop suggestions
      ↓
preview coverage
      ↓
commit
      ↓
vehicles spawn
      ↓
ridership emerges
```

---

# 68. Long-Term Emergence Goal

The most important architectural property is that the city should be able to produce outcomes that were not explicitly scripted as individual events.

The intended causal structure is:

```text
Player decisions
       ↓
Infrastructure
       ↓
Accessibility
       ↓
Land values
       ↓
Household/business decisions
       ↓
Economic activity
       ↓
Employment / migration
       ↓
Traffic / services / environment
       ↓
Neighborhood evolution
       ↓
Political pressure
       ↓
New player decisions
```

The game becomes deep because systems interact, not because the UI contains hundreds of buttons.

---

# 69. Architectural North Star

The project should ultimately behave like a **living urban system** rather than a building-placement game.

The player should be able to create a surprisingly simple initial intervention:

> “Build a road from here to there and zone this district for housing.”

Then the simulation should create a chain of consequences:

```text
road
 → accessibility
 → land value
 → developers
 → buildings
 → residents
 → jobs
 → commuting
 → traffic
 → transit demand
 → tax revenue
 → service demand
 → political pressure
 → policy change
 → further development
```

This causal depth is the real equivalent of a large content catalog.

---

# 70. External Technical Notes

WebGPU is a good preferred rendering/compute target for this project because it is designed for high-performance GPU graphics and computation in the browser; however, current browser availability remains uneven, so the renderer must retain a WebGL2 fallback. citeturn856704search1turn856704search7

For lightweight client-side learned models, ONNX Runtime Web provides browser inference through WebAssembly and GPU-oriented execution paths including WebGPU. This makes it practical to keep some prediction workloads on-device while leaving larger workloads on the server. citeturn856704search10turn856704search0

SQLite has an official WebAssembly/JavaScript project, making it a reasonable fit for a single-player-first persistence strategy and browser-side storage utilities. citeturn856704search3turn856704search6

---

# 71. Immediate Next Implementation Step

Do **not** start by implementing citizens, AI, traffic, or a giant UI.

The first implementation milestone should be the smallest vertical slice that proves the architecture:

```text
procedural terrain
      ↓
road drawing
      ↓
parcel generation
      ↓
zoning
      ↓
procedural buildings
      ↓
basic population
      ↓
basic economy
      ↓
save/reload
      ↓
browser rendering
```

Once this loop works, every later subsystem can attach to it without requiring a rewrite of the core interaction model.

The first playable prototype should therefore answer one question:

> **Can a player draw a road, zone an area, watch a believable city emerge, inspect why it is changing, save it, and come back later?**

If the answer is yes, the architecture is ready to scale toward the deeper simulation.
