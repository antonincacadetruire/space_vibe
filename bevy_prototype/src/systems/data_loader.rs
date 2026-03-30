// systems/data_loader.rs
// Loads map and skin definitions from JSON files in data/maps/ and data/skins/
// at game startup. The data is presented as Bevy resources so that UI and
// scene systems can query them without re-reading the filesystem.

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;

// ── JSON-compatible structs ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct MapDef {
    pub id: String,
    pub label: String,
    pub description: String,
    pub boundary_radius: f32,
    // accent_color is exported for use by future UI theming.
    #[allow(dead_code)]
    pub accent_color: [f32; 3],
    pub preview_svg: String,
}

/// A single geometric primitive used in a composable skin.
/// Ship coordinate system: -Z = forward (nose), +Z = tail, ±X = wings, +Y = up.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SkinPart {
    /// Primitive: "sphere" | "icosphere" | "box" | "cylinder" | "capsule" | "torus" | "cone"
    pub shape: String,
    /// Center position [x, y, z] in ship-local space.
    #[serde(default)]
    pub pos: Option<[f32; 3]>,
    /// Euler rotation in DEGREES [rx, ry, rz] applied in XYZ order.
    #[serde(default)]
    pub rot: Option<[f32; 3]>,
    /// Non-uniform scale [x, y, z] — default [1,1,1]. Use to squash/stretch primitives.
    #[serde(default)]
    pub scale: Option<[f32; 3]>,
    /// Named color slot: "hull" (primary_color) | "accent" (secondary_color) | "glow" (emissive_color).
    /// Overridden by color_rgb when both are set.
    #[serde(default)]
    pub color: String,
    /// Explicit base color [r, g, b] in 0–1 range. Overrides the named `color` slot.
    #[serde(default)]
    pub color_rgb: Option<[f32; 3]>,
    /// Explicit emissive color [r, g, b] in 0–1 range. Makes this part glow independently.
    #[serde(default)]
    pub emissive_rgb: Option<[f32; 3]>,
    /// Material metallic factor (0–1). Falls back to the slot default when absent.
    #[serde(default)]
    pub metallic: Option<f32>,
    /// Material roughness factor (0–1). Falls back to the slot default when absent.
    #[serde(default)]
    pub roughness: Option<f32>,
    /// Radius — sphere / icosphere / cylinder / capsule / torus / cone.
    #[serde(default)]
    pub radius: Option<f32>,
    /// Height — cylinder / capsule / cone.
    #[serde(default)]
    pub height: Option<f32>,
    /// Tube radius — torus.
    #[serde(default)]
    pub ring_radius: Option<f32>,
    /// Full extents [width, height, depth] — box, centered at pos.
    #[serde(default)]
    pub size: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SkinDef {
    pub id: String,
    pub label: String,
    pub description: String,
    pub preview_svg: String,
    /// Simple shape preset — ignored when `parts` is non-empty.
    #[serde(default)]
    pub shape: String,
    /// Main hull colour [r, g, b] in 0.0–1.0 range.
    #[serde(default)]
    pub primary_color: Option<[f32; 3]>,
    /// Accent / wing colour [r, g, b] in 0.0–1.0 range.
    #[serde(default)]
    pub secondary_color: Option<[f32; 3]>,
    /// Engine-glow emissive colour [r, g, b] in 0.0–1.0 range.
    #[serde(default)]
    pub emissive_color: Option<[f32; 3]>,
    /// Composable part list.  When non-empty, `shape` is ignored.
    #[serde(default)]
    pub parts: Vec<SkinPart>,
}

// ── Bevy resources ─────────────────────────────────────────────────────────────

/// All loaded map definitions, ordered as they appear in data/maps/.
/// The maps are loaded in alphabetical filename order then sorted to match the
/// canonical ordering: space_asteroids → ice_caves → desert_planet.
#[derive(Resource, Default, Debug, Clone)]
pub struct MapCatalog {
    pub maps: Vec<MapDef>,
}

impl MapCatalog {
    pub fn load() -> Self {
        let data_dir = data_dir().join("maps");
        let mut maps = load_json_dir::<MapDef>(&data_dir);

        // Ensure canonical ordering
        let order = ["space_asteroids", "ice_caves", "desert_planet"];
        maps.sort_by(|a, b| {
            let ai = order.iter().position(|&x| x == a.id).unwrap_or(99);
            let bi = order.iter().position(|&x| x == b.id).unwrap_or(99);
            ai.cmp(&bi)
        });

        if maps.is_empty() {
            warn!("No map definitions found in {:?}. Using fallback.", data_dir);
        }
        MapCatalog { maps }
    }

    pub fn by_id(&self, id: &str) -> Option<&MapDef> {
        self.maps.iter().find(|m| m.id == id)
    }
}

/// All loaded skin definitions.
#[derive(Resource, Default, Debug, Clone)]
pub struct SkinCatalog {
    pub skins: Vec<SkinDef>,
}

impl SkinCatalog {
    pub fn load() -> Self {
        let data_dir = data_dir().join("skins");
        let mut skins = load_json_dir::<SkinDef>(&data_dir);

        let order = ["war_plane", "banana", "mosquito", "butterfly", "grapefruit", "mushroom_skin", "artichoke_skin", "big_old_flower", "stick_skin_01"];
        skins.sort_by(|a, b| {
            let ai = order.iter().position(|&x| x == a.id).unwrap_or(99);
            let bi = order.iter().position(|&x| x == b.id).unwrap_or(99);
            ai.cmp(&bi)
        });

        if skins.is_empty() {
            warn!("No skin definitions found in {:?}. Using fallback.", data_dir);
        }
        SkinCatalog { skins }
    }

    pub fn by_id(&self, id: &str) -> Option<&SkinDef> {
        self.skins.iter().find(|s| s.id == id)
    }
}

// ── Enemy definitions ──────────────────────────────────────────────────────────

/// Full definition of an enemy type, loaded from `data/enemies/<id>.json`.
/// All spawn parameters and visual colours are customisable per-file.
#[derive(Debug, Clone, Deserialize)]
pub struct EnemyDef {
    pub id: String,
    pub label: String,
    pub description: String,
    /// Main hull base colour [R, G, B] (0-1 range, can exceed 1 for HDR).
    pub hull_color: [f32; 3],
    /// Hull emissive glow [R, G, B].
    pub hull_emissive: [f32; 3],
    /// Outer rim base colour [R, G, B].
    pub rim_color: [f32; 3],
    /// Outer rim emissive glow [R, G, B].
    pub rim_emissive: [f32; 3],
    /// Dome base colour [R, G, B].
    pub dome_color: [f32; 3],
    /// Dome emissive glow [R, G, B].
    pub dome_emissive: [f32; 3],
    /// Minimum flight speed (units/s).
    pub speed_min: f32,
    /// Maximum flight speed (units/s).
    pub speed_max: f32,
    /// Hit-points the enemy has.
    pub health: i32,
    /// Minimum seconds between shots.
    pub shoot_interval_min: f32,
    /// Maximum seconds between shots.
    pub shoot_interval_max: f32,
    /// How many seconds after the game starts before the first spawn.
    pub first_spawn_time: f32,
    /// Maximum simultaneous enemies of this type in the world.
    pub max_count: usize,
    /// Seconds between individual spawn events.
    pub spawn_interval: f32,
    /// Minimum spawn distance from the player (units).
    pub spawn_dist_min: f32,
    /// Maximum spawn distance from the player (units).
    pub spawn_dist_max: f32,
    /// SVG string used to render a thumbnail in the UI.
    pub preview_svg: String,
}

/// All loaded enemy definitions, first entry is used as the active enemy type.
#[derive(Resource, Default, Debug, Clone)]
pub struct EnemyCatalog {
    pub enemies: Vec<EnemyDef>,
}

impl EnemyCatalog {
    pub fn load() -> Self {
        let data_dir = data_dir().join("enemies");
        let enemies = load_json_dir::<EnemyDef>(&data_dir);
        if enemies.is_empty() {
            warn!("No enemy definitions found in {:?}. Using defaults.", data_dir);
        }
        EnemyCatalog { enemies }
    }

    /// Returns the first enemy def, or panics if the list is empty.
    /// Callers should always ensure at least one enemy is loaded (fallback
    /// should be inserted by `load_catalogs`).
    pub fn active(&self) -> &EnemyDef {
        self.enemies.first().expect("EnemyCatalog is empty – fallback was not inserted")
    }

    pub fn by_id(&self, id: &str) -> Option<&EnemyDef> {
        self.enemies.iter().find(|e| e.id == id)
    }
}

// ── LLM / Copilot chat configuration ──────────────────────────────────────────

/// Configuration for the in-game AI chat, loaded from `data/llm_config.json`.
/// Supports any OpenAI-compatible API endpoint (GitHub Copilot, OpenAI, Ollama …).
#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    /// Full URL of the chat completions endpoint.
    pub api_url: String,
    /// Bearer token / API key (leave empty to omit the Authorization header).
    pub api_key: String,
    /// Model name sent in the request body.
    pub model: String,
    /// System prompt that instructs the model what to generate.
    pub system_prompt: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        LlmConfig {
            api_url: "https://models.inference.ai.azure.com/chat/completions".into(),
            api_key: String::new(),
            model: "gpt-4o".into(),
            system_prompt: concat!(
                "You are SpaceVibe Copilot, an AI companion for a 3-D space-shooter game built with Bevy (Rust).\n",
                "Chat freely with the player: answer questions, give flying tips, discuss lore, or just hang out.\n",
                "IMPORTANT — SKIN GENERATION RULE: When the player mentions any object, creature, vehicle, food, weapon, ",
                "tool, or concept as a skin (e.g. 'gun skin', 'sword skin', 'pizza skin', 'cat skin', 'bottle skin', etc.), ",
                "ALWAYS interpret it as 'make a ship that LOOKS LIKE that object' using the SKIN (parts) schema. ",
                "Never refuse or say you can't — every real-world shape can be approximated with spheres, boxes, cylinders, capsules, cones and toruses. ",
                "A gun skin = a ship shaped like a pistol. A sword skin = a ship shaped like a sword. Always try.\n",
                "Only when the player explicitly asks you to GENERATE or CREATE a map, skin, or enemy, ",
                "respond with ONLY a single valid JSON object inside a ```json ... ``` code block, using one of these schemas:\n",
                "  MAP:   {\"id\":\"...\",\"label\":\"...\",\"description\":\"...\",\"boundary_radius\":float,\"accent_color\":[r,g,b],\"preview_svg\":\"<svg>...</svg>\"}\n",
                "  SKIN (simple): {\"id\":\"...\",\"label\":\"...\",\"description\":\"...\",",
                "\"shape\":\"sphere|disc|diamond|organic|cylinder\",",
                "\"primary_color\":[r,g,b],\"secondary_color\":[r,g,b],\"emissive_color\":[r,g,b],",
                "\"preview_svg\":\"<svg.../>\"}\n",
                "  SKIN (parts):  {\"id\":\"...\",\"label\":\"...\",\"description\":\"...\",",
                "\"primary_color\":[r,g,b],\"secondary_color\":[r,g,b],\"emissive_color\":[r,g,b],",
                "\"preview_svg\":\"<svg.../>\",",
                "\"parts\":[{\"shape\":\"sphere|icosphere|box|cylinder|capsule|torus|cone\",",
                "\"pos\":[x,y,z],\"rot\":[rx_deg,ry_deg,rz_deg],\"scale\":[sx,sy,sz],",
                "\"color\":\"hull|accent|glow\",\"color_rgb\":[r,g,b],\"emissive_rgb\":[r,g,b],",
                "\"metallic\":float,\"roughness\":float,",
                "\"radius\":float,\"height\":float,\"ring_radius\":float,\"size\":[w,h,d]},...]}\n",
                "  ENEMY: {\"id\":\"...\",\"label\":\"...\",\"description\":\"...\",\"hull_color\":[r,g,b],\"hull_emissive\":[r,g,b],\"rim_color\":[r,g,b],\"rim_emissive\":[r,g,b],\"dome_color\":[r,g,b],\"dome_emissive\":[r,g,b],",
                "\"speed_min\":float,\"speed_max\":float,\"health\":int,\"shoot_interval_min\":float,\"shoot_interval_max\":float,",
                "\"first_spawn_time\":float,\"max_count\":int,\"spawn_interval\":float,\"spawn_dist_min\":float,\"spawn_dist_max\":float,\"preview_svg\":\"<svg>...</svg>\"}\n",
                "Ship coords for parts: -Z=nose/forward, +Z=tail, ±X=wings, +Y=up. Simple shapes: sphere,disc,diamond,organic,cylinder. Parts primitives: sphere(radius), icosphere(radius), box(size:[w,h,d]), cylinder(radius,height), capsule(radius,height), torus(radius,ring_radius), cone(radius,height — apex at +Y, base at -Y; use rot:[-90,0,0] to point nose forward). rot is Euler degrees [rx,ry,rz] applied XYZ. scale can squash/stretch any primitive. color is hull|accent|glow or use color_rgb:[r,g,b] for explicit per-part color; emissive_rgb adds emission to that part only. Use parts to compose complex ships (butterfly, grapefruit, gun, sword, bottle, animal, etc.).\n",
                "Colors are [r,g,b] floats in 0.0-1.0 range. Pick vivid, thematic colors.\n",
                "You can also issue GAME COMMANDS by including a [CMD: command_name arg] token anywhere in your reply. ",
                "The player will be asked to confirm before the command executes. Available commands:\n",
                "  [CMD: set_speed <value>]     — change the player's max flight speed (default 40000)\n",
                "  [CMD: set_boundary <radius>] — resize the play zone (default 100000)\n",
                "  [CMD: teleport_origin]       — warp the player back to the centre\n",
                "For all other messages, reply in plain text only \u{2014} no JSON."
            ).into(),
        }
    }
}

#[derive(Resource, Default, Debug, Clone)]
pub struct LlmConfigResource(pub LlmConfig);

// ── Current selection indices (carousel state) ────────────────────────────────
#[derive(Resource, Default)]
pub struct CarouselState {
    pub map_idx: usize,
    pub skin_idx: usize,
}

// ── Rasterised previews (parallel indexing with MapCatalog / SkinCatalog) ─────

/// Pre-rasterised SVG previews for all maps. `handles[i]` corresponds to
/// `MapCatalog::maps[i]`.
#[derive(Resource, Default)]
pub struct MapCatalogImages {
    pub handles: Vec<Handle<Image>>,
}

/// Pre-rasterised SVG previews for all skins. `handles[i]` corresponds to
/// `SkinCatalog::skins[i]`.
#[derive(Resource, Default)]
pub struct SkinCatalogImages {
    pub handles: Vec<Handle<Image>>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the `data/` directory relative to the game executable.
fn data_dir() -> std::path::PathBuf {
    // When running with `cargo run` the cwd is the project root, so
    // `data/` lives there. In a release build it lives beside the exe.
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("data")));

    let cwd_path = std::path::PathBuf::from("data");

    if cwd_path.exists() {
        cwd_path
    } else if let Some(p) = beside_exe {
        p
    } else {
        std::path::PathBuf::from("data")
    }
}

fn load_json_dir<T: for<'de> Deserialize<'de>>(dir: &std::path::Path) -> Vec<T> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        warn!("Could not read directory {:?}", dir);
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut paths: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
        .collect();
    paths.sort();

    for path in paths {
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<T>(&text) {
                Ok(v) => result.push(v),
                Err(e) => warn!("Failed to parse {:?}: {}", path, e),
            },
            Err(e) => warn!("Could not read {:?}: {}", path, e),
        }
    }
    result
}

/// Startup system — loads catalogs into Bevy resources.
pub fn load_catalogs(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let map_catalog = MapCatalog::load();
    let skin_catalog = SkinCatalog::load();
    let mut enemy_catalog = EnemyCatalog::load();

    // Ensure at least one enemy definition exists.
    if enemy_catalog.enemies.is_empty() {
        enemy_catalog.enemies.push(default_enemy_def());
    }

    // Set alien spawn timer from the active enemy's configuration.
    let spawn_interval = enemy_catalog.active().spawn_interval;
    commands.insert_resource(
        crate::resources::AlienSpawnTimer(bevy::time::Timer::from_seconds(
            spawn_interval,
            bevy::time::TimerMode::Repeating,
        )),
    );

    // Load LLM config (optional – fall back to defaults if missing).
    let llm_config = load_llm_config();

    // Rasterise SVG previews (128×128 thumbnails).
    const THUMB: u32 = 128;

    let map_handles: Vec<Handle<Image>> = map_catalog.maps.iter()
        .map(|m| images.add(svg_to_image(&m.preview_svg, THUMB, THUMB)))
        .collect();

    let skin_handles: Vec<Handle<Image>> = skin_catalog.skins.iter()
        .map(|s| images.add(svg_to_image(&s.preview_svg, THUMB, THUMB)))
        .collect();

    commands.insert_resource(map_catalog);
    commands.insert_resource(skin_catalog);
    commands.insert_resource(enemy_catalog);
    commands.insert_resource(LlmConfigResource(llm_config));
    commands.insert_resource(CarouselState::default());
    commands.insert_resource(MapCatalogImages { handles: map_handles });
    commands.insert_resource(SkinCatalogImages { handles: skin_handles });
}

/// Constructs a hard-coded fallback enemy definition used when no JSON files
/// are found in `data/enemies/`.
fn default_enemy_def() -> EnemyDef {
    EnemyDef {
        id: "alien_ufo".into(),
        label: "Alien UFO".into(),
        description: "Classic flying saucer – glows purple and fires plasma bolts.".into(),
        hull_color:   [0.10, 0.04, 0.20],
        hull_emissive:[1.50, 0.00, 3.00],
        rim_color:    [0.02, 0.40, 0.60],
        rim_emissive: [0.00, 8.00, 12.0],
        dome_color:   [0.05, 0.90, 0.30],
        dome_emissive:[0.00, 3.00, 0.60],
        speed_min: 1_400.0,
        speed_max: 2_200.0,
        health: 3,
        shoot_interval_min: 3.5,
        shoot_interval_max: 5.5,
        first_spawn_time: 25.0,
        max_count: 12,
        spawn_interval: 30.0,
        spawn_dist_min: 7_000.0,
        spawn_dist_max: 12_000.0,
        preview_svg: "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>\
            <ellipse cx='50' cy='55' rx='40' ry='14' fill='#280850' stroke='#9030f0' stroke-width='2'/>\
            <ellipse cx='50' cy='50' rx='22' ry='8' fill='#3a0a60'/>\
            <ellipse cx='50' cy='47' rx='12' ry='10' fill='#08e050' opacity='0.8'/>\
            <circle cx='50' cy='55' r='3' fill='#00cfff' opacity='0.9'/>\
            </svg>".into(),
    }
}

/// Loads `data/llm_config.json`, falling back to `LlmConfig::default()`.
/// Then overlays `api_key` from `data/secrets.json` if that file exists and
/// contains a non-empty key.  The placeholder string "YOUR_GITHUB_PAT_HERE"
/// is treated as empty so it never sends a bogus Authorization header.
fn load_llm_config() -> LlmConfig {
    let path = data_dir().join("llm_config.json");
    let mut cfg = match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<LlmConfig>(&text) {
            Ok(cfg) => {
                info!("Loaded LLM config from {:?}", path);
                cfg
            }
            Err(e) => {
                warn!("Could not parse {:?}: {}. Using defaults.", path, e);
                LlmConfig::default()
            }
        },
        Err(_) => {
            info!("No llm_config.json found – using defaults (set data/llm_config.json to configure AI chat).");
            LlmConfig::default()
        }
    };

    // Treat the placeholder as empty
    if cfg.api_key == "YOUR_GITHUB_PAT_HERE" {
        cfg.api_key.clear();
    }

    // Overlay from data/secrets.json (preferred storage for the actual key)
    let secrets_path = data_dir().join("secrets.json");
    if let Ok(text) = std::fs::read_to_string(&secrets_path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(key) = v.get("api_key").and_then(|k| k.as_str()) {
                if !key.is_empty() {
                    cfg.api_key = key.to_owned();
                    info!("Loaded api_key from {:?}", secrets_path);
                }
            }
        }
    }

    cfg
}

// ── SVG → Bevy Image ─────────────────────────────────────────────────────────

/// Rasterises an SVG string into a `RGBA8UnormSrgb` Bevy [`Image`].
/// Returns a solid-grey placeholder on error so the UI never panics.
pub fn svg_to_image(svg: &str, width: u32, height: u32) -> Image {
    use resvg::{usvg, tiny_skia};
    use resvg::usvg::TreeParsing;

    let fallback = || {
        let data = vec![40u8, 40, 40, 255].repeat((width * height) as usize);
        Image::new(
            Extent3d { width, height, depth_or_array_layers: 1 },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
        )
    };

    let Ok(utree) = usvg::Tree::from_str(svg, &usvg::Options::default()) else {
        return fallback();
    };

    let rtree = resvg::Tree::from_usvg(&utree);

    let Some(mut pixmap) = tiny_skia::Pixmap::new(width, height) else {
        return fallback();
    };

    let svg_size = rtree.size;
    let transform = tiny_skia::Transform::from_scale(
        width  as f32 / svg_size.width()  as f32,
        height as f32 / svg_size.height() as f32,
    );
    rtree.render(transform, &mut pixmap.as_mut());

    Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixmap.data().to_vec(),
        TextureFormat::Rgba8UnormSrgb,
    )
}
