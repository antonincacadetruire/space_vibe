//! Île-de-France transport network map — Parisian environment.
//!
//! 3-D model of the Paris metro/RER/Transilien network at 3× real scale.
//! Stations are glowing spheres (gold = selected, line-coloured = normal).
//! Enemy trains patrol their lines and shoot homing missiles at the player.
//! Approach a selected station to see upcoming departures displayed above it.
//!
//! Visual environment:
//! - Overcast Parisian grey sky dome with scattered cloud patches.
//! - Green suburban ground outside Paris, beige city ground inside the périphérique.
//! - Animated blue Seine river winding through the city centre.
//! - Tour Eiffel and Arc de Triomphe as recognisable geometric landmarks.

use bevy::prelude::*;
use rand::Rng;
use std::sync::{Arc, Mutex};

use crate::components::{MainCamera, Missile, SceneEntity};
use crate::resources::{
    ActiveScene, CameraArmOffset, DeathCause, GameState, GameTimer,
    IdfConfig, IdfNextTrains, IdfTerrainData, SceneKind, TimePaused,
};

// ─────────────────────────────────────────────────────────────────────────────
// Static network data (positions scaled: 1 unit ≈ 250 m real distance)
// ─────────────────────────────────────────────────────────────────────────────

/// All IDF lines known to SpaceVibe, with display colour and Y-elevation offset
/// so that overlapping lines are visually separated in 3D.
///   (id,  label,  [r,g,b],  y_offset)
pub const IDF_LINES: &[(&str, &str, [f32; 3], f32)] = &[
    ("M1",    "Métro 1",       [0.99, 0.82, 0.20],   0.0),
    ("M2",    "Métro 2",       [0.10, 0.25, 0.75],  30.0),
    ("M3",    "Métro 3",       [0.60, 0.55, 0.10],  60.0),
    ("M4",    "Métro 4",       [0.70, 0.18, 0.50],  90.0),
    ("M5",    "Métro 5",       [0.90, 0.55, 0.20], 120.0),
    ("M6",    "Métro 6",       [0.53, 0.80, 0.45], 150.0),
    ("M7",    "Métro 7",       [0.90, 0.60, 0.70], 180.0),
    ("M8",    "Métro 8",       [0.75, 0.60, 0.85], 210.0),
    ("M9",    "Métro 9",       [0.80, 0.78, 0.10], 240.0),
    ("M10",   "Métro 10",      [0.85, 0.65, 0.20], 270.0),
    ("M11",   "Métro 11",      [0.55, 0.35, 0.15], 300.0),
    ("M12",   "Métro 12",      [0.10, 0.55, 0.30], 330.0),
    ("M13",   "Métro 13",      [0.44, 0.78, 0.74], 360.0),
    ("M14",   "Métro 14",      [0.55, 0.10, 0.65], 390.0),
    ("RER_A", "RER A",         [0.90, 0.20, 0.20], 450.0),
    ("RER_B", "RER B",         [0.18, 0.45, 0.80], 500.0),
    ("RER_C", "RER C",         [0.85, 0.75, 0.10], 550.0),
    ("RER_D", "RER D",         [0.15, 0.60, 0.20], 600.0),
    ("RER_E", "RER E",         [0.85, 0.50, 0.10], 650.0),
];

/// Marker component for an IDF station cube entity.
#[derive(Component)]
pub struct IdfStation {
    pub station_idx: usize,
}

/// Marker component for an IDF enemy train entity.
#[derive(Component)]
pub struct IdfTrain {
    pub line_id: &'static str,
    /// World-space waypoints the train patrols between.
    pub waypoints: Vec<Vec3>,
    pub current_wp: usize,
    pub speed: f32,
    /// Countdown in seconds until next missile shot.
    pub shoot_timer: f32,
    /// Seconds between missile shots (randomised per train).
    pub shoot_interval: f32,
}

/// Bottom-left HUD entity: shows next departures as plain text.
#[derive(Component)]
pub struct IdfProximityHud;

/// Screen-projected popup: positioned above the nearest selected station.
#[derive(Component)]
pub struct IdfProximityPopup;

/// Screen-projected label parented to a train: shows line name near the train.
#[derive(Component)]
pub struct IdfTrainLabel {
    /// Entity ID of the IdfTrain this label tracks.
    pub target: Entity,
}

// ── Station catalogue ─────────────────────────────────────────────────────────
//   id, label, prim_id (IDFM Logical Stop Point), lines, [x, y, z]
//   x = east-west  (positive = east),  z = north-south (negative = north)
//   y is 0 everywhere; per-line elevation is added at render time.
//   All positions relative to Notre-Dame de Paris ≈ (0, 0, 0).
//   1 unit ≈ 250 m real distance.
pub const IDF_STATIONS: &[(&str, &str, &str, &[&str], [f32; 3])] = &[
    // ═══════════════════════ RER A ═══════════════════════════════════════════
    ("rera_sgermain",    "Saint-Germain-en-Laye","IDFM:monomodalStopPlace:71560", &["RER_A"],      [-160.0, 0.0, -50.0]),
    ("rera_vesinet",     "Le Vésinet-Le Pecq",   "IDFM:monomodalStopPlace:71562", &["RER_A"],      [-140.0, 0.0, -48.0]),
    ("rera_chatou",      "Chatou-Croissy",       "IDFM:monomodalStopPlace:71563", &["RER_A"],      [-125.0, 0.0, -45.0]),
    ("rera_rueil",       "Rueil-Malmaison",      "IDFM:monomodalStopPlace:71565", &["RER_A"],      [-110.0, 0.0, -40.0]),
    ("rera_nanterre",    "Nanterre-Préfecture",  "IDFM:monomodalStopPlace:71570", &["RER_A"],      [-100.0, 0.0, -38.0]),
    ("rera_la_defense",  "La Défense",           "IDFM:monomodalStopPlace:71575", &["RER_A","M1"], [ -80.0, 0.0, -32.0]),
    ("rera_charles_de_gaulle","Charles-de-Gaulle–Étoile","IDFM:monomodalStopPlace:71580",&["RER_A","M1","M2","M6"],[-30.0, 0.0,-28.0]),
    ("rera_auber",       "Auber / Opéra",        "IDFM:monomodalStopPlace:71585", &["RER_A","M3","M7","M8"],[-12.0, 0.0,-20.0]),
    ("rera_chatelet",    "Châtelet–Les Halles",  "IDFM:monomodalStopPlace:71590", &["RER_A","RER_B","RER_D","M1","M4","M7","M11","M14"],[0.0, 0.0, 0.0]),
    ("rera_gare_de_lyon","Gare de Lyon",         "IDFM:monomodalStopPlace:71596", &["RER_A","RER_D","M1","M14"],[30.0, 0.0, 15.0]),
    ("rera_nation",      "Nation",               "IDFM:monomodalStopPlace:71600", &["RER_A","M1","M2","M6","M9"],[45.0, 0.0, 10.0]),
    ("rera_vincennes",   "Vincennes",            "IDFM:monomodalStopPlace:71601", &["RER_A"],      [ 65.0, 0.0,  8.0]),
    ("rera_noisy",       "Noisy-le-Grand",       "IDFM:monomodalStopPlace:71605", &["RER_A"],      [120.0, 0.0, 15.0]),
    ("rera_marne",       "Marne-la-Vallée / Chessy","IDFM:monomodalStopPlace:72100",&["RER_A"],    [200.0, 0.0, 20.0]),
    // ═══════════════════════ RER B ═══════════════════════════════════════════
    ("rerb_cdg2",        "CDG Terminal 2",       "IDFM:monomodalStopPlace:70071", &["RER_B"],      [140.0, 0.0,-320.0]),
    ("rerb_cdg1",        "CDG Terminal 1",       "IDFM:monomodalStopPlace:70072", &["RER_B"],      [135.0, 0.0,-310.0]),
    ("rerb_parc_expo",   "Parc des Expositions", "IDFM:monomodalStopPlace:43046", &["RER_B"],      [100.0, 0.0,-280.0]),
    ("rerb_villepinte",  "Villepinte",           "IDFM:monomodalStopPlace:43047", &["RER_B"],      [ 90.0, 0.0,-260.0]),
    ("rerb_sevran",      "Sevran-Beaudottes",    "IDFM:monomodalStopPlace:43048", &["RER_B"],      [ 75.0, 0.0,-245.0]),
    ("rerb_aulnay",      "Aulnay-sous-Bois",    "IDFM:monomodalStopPlace:43045", &["RER_B"],      [ 70.0, 0.0,-235.0]),
    ("rerb_blanc_mesnil","Le Blanc-Mesnil",      "IDFM:monomodalStopPlace:43044", &["RER_B"],      [ 60.0, 0.0,-230.0]),
    ("rerb_drancy",      "Drancy",               "IDFM:monomodalStopPlace:43042", &["RER_B"],      [ 50.0, 0.0,-200.0]),
    ("rerb_bourget",     "Le Bourget",           "IDFM:monomodalStopPlace:43050", &["RER_B"],      [ 45.0, 0.0,-185.0]),
    ("rerb_courneuve",   "La Courneuve 8 Mai",   "IDFM:monomodalStopPlace:43041", &["RER_B"],      [ 38.0, 0.0,-165.0]),
    ("rerb_stade_france","Stade de France",      "IDFM:monomodalStopPlace:43039", &["RER_B"],      [ 25.0, 0.0,-150.0]),
    ("rerb_saint_denis", "Saint-Denis",          "IDFM:monomodalStopPlace:43038", &["RER_B","M13"],[ 21.0, 0.0,-140.0]),
    ("rerb_la_plaine",   "La Plaine–St-Denis",   "IDFM:monomodalStopPlace:43037", &["RER_B"],      [ 18.0, 0.0,-125.0]),
    ("rerb_gare_du_nord","Gare du Nord",         "IDFM:monomodalStopPlace:43003", &["RER_B","RER_D","RER_E","M4","M5"],[12.0, 0.0,-80.0]),
    ("rerb_chatelet",    "Châtelet–Les Halles",  "IDFM:monomodalStopPlace:43007", &["RER_B"],      [  0.0, 0.0,  0.0]),
    ("rerb_luxembourg",  "Luxembourg",           "IDFM:monomodalStopPlace:43009", &["RER_B"],      [ -2.0, 0.0, 20.0]),
    ("rerb_port_royal",  "Port-Royal",           "IDFM:monomodalStopPlace:43011", &["RER_B"],      [ -3.0, 0.0, 35.0]),
    ("rerb_denfert",     "Denfert-Rochereau",    "IDFM:monomodalStopPlace:43015", &["RER_B","M4","M6"],[-4.0, 0.0, 50.0]),
    ("rerb_cite_u",      "Cité Universitaire",   "IDFM:monomodalStopPlace:43017", &["RER_B"],      [ -4.0, 0.0, 75.0]),
    ("rerb_gentilly",    "Gentilly",             "IDFM:monomodalStopPlace:43021", &["RER_B"],      [ -5.0, 0.0, 95.0]),
    ("rerb_laplace",     "Laplace",              "IDFM:monomodalStopPlace:43023", &["RER_B"],      [ -5.0, 0.0,115.0]),
    ("rerb_arcueil",     "Arcueil–Cachan",       "IDFM:monomodalStopPlace:43025", &["RER_B"],      [ -6.0, 0.0,130.0]),
    ("rerb_bagneux",     "Bagneux",              "IDFM:monomodalStopPlace:43026", &["RER_B"],      [ -6.0, 0.0,150.0]),
    ("rerb_bourg",       "Bourg-la-Reine",       "IDFM:monomodalStopPlace:43027", &["RER_B"],      [ -7.0, 0.0,175.0]),
    ("rerb_antony",      "Antony",               "IDFM:monomodalStopPlace:43029", &["RER_B"],      [ -7.0, 0.0,195.0]),
    ("rerb_massy",       "Massy-Palaiseau",      "IDFM:monomodalStopPlace:43030", &["RER_B","RER_C"],[-30.0, 0.0,230.0]),
    ("rerb_robinson",    "Robinson",             "IDFM:monomodalStopPlace:43031", &["RER_B"],      [-20.0, 0.0,225.0]),
    ("rerb_st_remy",     "Saint-Rémy-lès-Chevreuse","IDFM:monomodalStopPlace:43035",&["RER_B"],    [-80.0, 0.0,260.0]),
    // ═══════════════════════ RER C ═══════════════════════════════════════════
    ("rerc_versailles",  "Versailles-Chantiers", "IDFM:monomodalStopPlace:41324", &["RER_C"],      [-90.0, 0.0, 60.0]),
    ("rerc_porchefontaine","Porchefontaine",     "IDFM:monomodalStopPlace:41325", &["RER_C"],      [-80.0, 0.0, 55.0]),
    ("rerc_chaville",    "Chaville-Vélizy",      "IDFM:monomodalStopPlace:41330", &["RER_C"],      [-65.0, 0.0, 45.0]),
    ("rerc_issy",        "Issy",                 "IDFM:monomodalStopPlace:41340", &["RER_C"],      [-40.0, 0.0, 30.0]),
    ("rerc_invalides",   "Invalides",            "IDFM:monomodalStopPlace:41105", &["RER_C","M8","M13"],[-18.0, 0.0,-15.0]),
    ("rerc_musee_orsay", "Musée d'Orsay",        "IDFM:monomodalStopPlace:41106", &["RER_C"],      [-12.0, 0.0, -8.0]),
    ("rerc_st_michel",   "Saint-Michel–Notre-Dame","IDFM:monomodalStopPlace:41107",&["RER_C"],      [ -2.0, 0.0,  5.0]),
    ("rerc_austerlitz",  "Gare d'Austerlitz",    "IDFM:monomodalStopPlace:41108", &["RER_C","M5","M10"],[12.0, 0.0, 18.0]),
    ("rerc_ivry",        "Ivry-sur-Seine",       "IDFM:monomodalStopPlace:41115", &["RER_C"],      [ 25.0, 0.0, 45.0]),
    ("rerc_choisy",      "Choisy-le-Roi",        "IDFM:monomodalStopPlace:41120", &["RER_C"],      [ 30.0, 0.0, 80.0]),
    ("rerc_juvisy",      "Juvisy",               "IDFM:monomodalStopPlace:41130", &["RER_C","RER_D"],[ 32.0, 0.0,120.0]),
    // ═══════════════════════ RER D ═══════════════════════════════════════════
    ("rerd_creil",       "Creil",                "IDFM:monomodalStopPlace:40601", &["RER_D"],      [ 20.0, 0.0,-260.0]),
    ("rerd_orry",        "Orry-la-Ville",        "IDFM:monomodalStopPlace:40605", &["RER_D"],      [ 25.0, 0.0,-220.0]),
    ("rerd_goussainville","Goussainville",        "IDFM:monomodalStopPlace:40610", &["RER_D"],      [ 35.0, 0.0,-200.0]),
    ("rerd_villiers_le_bel","Villiers-le-Bel",    "IDFM:monomodalStopPlace:40615", &["RER_D"],      [ 30.0, 0.0,-180.0]),
    ("rerd_stade_france","Stade de France",       "IDFM:monomodalStopPlace:40620", &["RER_D"],      [ 25.0, 0.0,-150.0]),
    ("rerd_gare_du_nord","Gare du Nord",          "IDFM:monomodalStopPlace:40625", &["RER_D"],      [ 12.0, 0.0, -80.0]),
    ("rerd_chatelet",    "Châtelet–Les Halles",   "IDFM:monomodalStopPlace:40630", &["RER_D"],      [  0.0, 0.0,   0.0]),
    ("rerd_gare_de_lyon","Gare de Lyon",          "IDFM:monomodalStopPlace:40635", &["RER_D"],      [ 30.0, 0.0,  15.0]),
    ("rerd_maisons_alfort","Maisons-Alfort",      "IDFM:monomodalStopPlace:40640", &["RER_D"],      [ 40.0, 0.0,  50.0]),
    ("rerd_melun",       "Melun",                 "IDFM:monomodalStopPlace:40690", &["RER_D"],      [ 60.0, 0.0, 200.0]),
    // ═══════════════════════ RER E ═══════════════════════════════════════════
    ("rere_haussmann",   "Haussmann–St-Lazare",  "IDFM:monomodalStopPlace:45001", &["RER_E"],      [-15.0, 0.0,-45.0]),
    ("rere_magenta",     "Magenta (Gare du Nord)","IDFM:monomodalStopPlace:45002", &["RER_E"],      [ 12.0, 0.0,-80.0]),
    ("rere_rosa_parks",  "Rosa Parks",           "IDFM:monomodalStopPlace:45005", &["RER_E"],      [ 30.0, 0.0,-100.0]),
    ("rere_pantin",      "Pantin",               "IDFM:monomodalStopPlace:45010", &["RER_E"],      [ 40.0, 0.0,-110.0]),
    ("rere_noisy",       "Noisy-le-Sec",         "IDFM:monomodalStopPlace:45015", &["RER_E"],      [ 55.0, 0.0,-100.0]),
    ("rere_chelles",     "Chelles-Gournay",      "IDFM:monomodalStopPlace:45030", &["RER_E"],      [100.0, 0.0, -80.0]),
    ("rere_tournan",     "Tournan",              "IDFM:monomodalStopPlace:45050", &["RER_E"],      [170.0, 0.0, -50.0]),
    // ═══════════════════════ Métro 1 ═════════════════════════════════════════
    ("m1_la_defense",    "La Défense",            "IDFM:monomodalStopPlace:59501", &["M1"],         [-80.0, 0.0,-32.0]),
    ("m1_esplanade",     "Esplanade de La Défense","IDFM:monomodalStopPlace:59502",&["M1"],         [-68.0, 0.0,-32.0]),
    ("m1_neuilly",       "Les Sablons",           "IDFM:monomodalStopPlace:59504", &["M1"],         [-50.0, 0.0,-30.0]),
    ("m1_maillot",       "Porte Maillot",         "IDFM:monomodalStopPlace:59506", &["M1"],         [-38.0, 0.0,-30.0]),
    ("m1_etoile",        "Charles-de-Gaulle–Étoile","IDFM:monomodalStopPlace:59510",&["M1"],        [-30.0, 0.0,-28.0]),
    ("m1_george_v",      "George V",              "IDFM:monomodalStopPlace:59512", &["M1"],         [-22.0, 0.0,-25.0]),
    ("m1_champs_elysees","Champs-Élysées–Clemenceau","IDFM:monomodalStopPlace:59515",&["M1","M13"],[-18.0, 0.0,-22.0]),
    ("m1_concorde",      "Concorde",              "IDFM:monomodalStopPlace:59517", &["M1","M8","M12"],[-15.0, 0.0,-18.0]),
    ("m1_tuileries",     "Tuileries",             "IDFM:monomodalStopPlace:59518", &["M1"],         [-10.0, 0.0,-14.0]),
    ("m1_palais_royal",  "Palais Royal–Musée du Louvre","IDFM:monomodalStopPlace:59520",&["M1","M7"],[-6.0, 0.0,-10.0]),
    ("m1_chatelet",      "Châtelet",              "IDFM:monomodalStopPlace:59522", &["M1","M4","M7","M11","M14"],[0.0, 0.0, 0.0]),
    ("m1_hotel_de_ville","Hôtel de Ville",        "IDFM:monomodalStopPlace:59523", &["M1","M11"],   [  5.0, 0.0,  2.0]),
    ("m1_bastille",      "Bastille",              "IDFM:monomodalStopPlace:59525", &["M1","M5","M8"],[18.0, 0.0,  5.0]),
    ("m1_gare_de_lyon",  "Gare de Lyon",          "IDFM:monomodalStopPlace:59527", &["M1","M14"],   [ 30.0, 0.0, 15.0]),
    ("m1_nation",        "Nation",                "IDFM:monomodalStopPlace:59530", &["M1","M2","M6","M9"],[45.0, 0.0, 10.0]),
    ("m1_vincennes",     "Château de Vincennes",  "IDFM:monomodalStopPlace:59535", &["M1"],         [ 70.0, 0.0,  8.0]),
    // ═══════════════════════ Métro 2 ═════════════════════════════════════════
    ("m2_porte_dauphine","Porte Dauphine",        "IDFM:monomodalStopPlace:59550", &["M2"],         [-35.0, 0.0,-32.0]),
    ("m2_victor_hugo",   "Victor Hugo",           "IDFM:monomodalStopPlace:59551", &["M2"],         [-32.0, 0.0,-30.0]),
    ("m2_etoile",        "Charles-de-Gaulle–Étoile","IDFM:monomodalStopPlace:59552",&["M2"],        [-30.0, 0.0,-28.0]),
    ("m2_ternes",        "Ternes",                "IDFM:monomodalStopPlace:59553", &["M2"],         [-25.0, 0.0,-30.0]),
    ("m2_courcelles",    "Courcelles",            "IDFM:monomodalStopPlace:59554", &["M2"],         [-20.0, 0.0,-35.0]),
    ("m2_villiers",      "Villiers",              "IDFM:monomodalStopPlace:59555", &["M2","M3"],    [-16.0, 0.0,-40.0]),
    ("m2_rome",          "Rome",                  "IDFM:monomodalStopPlace:59556", &["M2"],         [-14.0, 0.0,-48.0]),
    ("m2_place_clichy",  "Place de Clichy",       "IDFM:monomodalStopPlace:59557", &["M2","M13"],   [-10.0, 0.0,-55.0]),
    ("m2_pigalle",       "Pigalle",               "IDFM:monomodalStopPlace:59560", &["M2","M12"],   [ -4.0, 0.0,-60.0]),
    ("m2_anvers",        "Anvers",                "IDFM:monomodalStopPlace:59561", &["M2"],         [  0.0, 0.0,-65.0]),
    ("m2_barbes",        "Barbès–Rochechouart",   "IDFM:monomodalStopPlace:59562", &["M2","M4"],    [  5.0, 0.0,-68.0]),
    ("m2_la_chapelle",   "La Chapelle",           "IDFM:monomodalStopPlace:59563", &["M2"],         [  8.0, 0.0,-75.0]),
    ("m2_stalingrad",    "Stalingrad",            "IDFM:monomodalStopPlace:59564", &["M2","M5","M7"],[12.0, 0.0,-82.0]),
    ("m2_jaures",        "Jaurès",                "IDFM:monomodalStopPlace:59565", &["M2","M5","M7bis"],[16.0, 0.0,-80.0]),
    ("m2_belleville",    "Belleville",            "IDFM:monomodalStopPlace:59570", &["M2","M11"],   [ 20.0, 0.0,-60.0]),
    ("m2_pere_lachaise", "Père Lachaise",         "IDFM:monomodalStopPlace:59572", &["M2","M3"],    [ 30.0, 0.0,-40.0]),
    ("m2_nation",        "Nation",                "IDFM:monomodalStopPlace:59575", &["M2"],         [ 45.0, 0.0, 10.0]),
    // ═══════════════════════ Métro 3 ═════════════════════════════════════════
    ("m3_pont_levallois","Pont de Levallois",     "IDFM:monomodalStopPlace:59580", &["M3"],         [-45.0, 0.0,-50.0]),
    ("m3_louise_michel", "Louise Michel",         "IDFM:monomodalStopPlace:59581", &["M3"],         [-38.0, 0.0,-48.0]),
    ("m3_villiers",      "Villiers",              "IDFM:monomodalStopPlace:59584", &["M3"],         [-16.0, 0.0,-40.0]),
    ("m3_st_lazare",     "Saint-Lazare",          "IDFM:monomodalStopPlace:59585", &["M3","M12","M13","M14"],[-15.0, 0.0,-45.0]),
    ("m3_opera",         "Opéra",                 "IDFM:monomodalStopPlace:59586", &["M3","M7","M8"],[-12.0, 0.0,-20.0]),
    ("m3_sentier",       "Sentier",               "IDFM:monomodalStopPlace:59588", &["M3"],         [ -4.0, 0.0,-15.0]),
    ("m3_republique",    "République",            "IDFM:monomodalStopPlace:59590", &["M3","M5","M8","M9","M11"],[10.0, 0.0,-30.0]),
    ("m3_pere_lachaise", "Père Lachaise",         "IDFM:monomodalStopPlace:59592", &["M3"],         [ 30.0, 0.0,-40.0]),
    ("m3_gallieni",      "Gallieni",              "IDFM:monomodalStopPlace:59595", &["M3"],         [ 55.0, 0.0,-30.0]),
    // ═══════════════════════ Métro 4 ═════════════════════════════════════════
    ("m4_clignancourt",  "Porte de Clignancourt", "IDFM:monomodalStopPlace:59600", &["M4"],         [  2.0, 0.0,-75.0]),
    ("m4_barbes",        "Barbès–Rochechouart",   "IDFM:monomodalStopPlace:59603", &["M4"],         [  5.0, 0.0,-68.0]),
    ("m4_gare_du_nord",  "Gare du Nord",          "IDFM:monomodalStopPlace:59605", &["M4","M5"],    [ 12.0, 0.0,-80.0]),
    ("m4_gare_de_lest",  "Gare de l'Est",         "IDFM:monomodalStopPlace:59606", &["M4","M5","M7"],[ 14.0, 0.0,-70.0]),
    ("m4_strasbourg",    "Strasbourg–Saint-Denis","IDFM:monomodalStopPlace:59607", &["M4","M8","M9"],[ 8.0, 0.0,-50.0]),
    ("m4_les_halles",    "Les Halles",            "IDFM:monomodalStopPlace:59608", &["M4"],         [  0.0, 0.0, -5.0]),
    ("m4_chatelet",      "Châtelet",              "IDFM:monomodalStopPlace:59610", &["M4"],         [  0.0, 0.0,  0.0]),
    ("m4_cite",          "Cité",                  "IDFM:monomodalStopPlace:59611", &["M4"],         [ -1.0, 0.0,  5.0]),
    ("m4_st_michel",     "Saint-Michel",          "IDFM:monomodalStopPlace:59612", &["M4"],         [ -2.0, 0.0, 10.0]),
    ("m4_odeon",         "Odéon",                 "IDFM:monomodalStopPlace:59613", &["M4","M10"],   [ -5.0, 0.0, 15.0]),
    ("m4_st_germain",    "Saint-Germain-des-Prés","IDFM:monomodalStopPlace:59614", &["M4"],         [ -8.0, 0.0, 18.0]),
    ("m4_montparnasse",  "Montparnasse–Bienvenüe","IDFM:monomodalStopPlace:59616", &["M4","M6","M12","M13"],[-12.0, 0.0, 30.0]),
    ("m4_denfert",       "Denfert-Rochereau",     "IDFM:monomodalStopPlace:59620", &["M4","M6"],    [ -4.0, 0.0, 50.0]),
    ("m4_mairie_montrouge","Mairie de Montrouge", "IDFM:monomodalStopPlace:59625", &["M4"],         [ -8.0, 0.0, 75.0]),
    ("m4_bagneux",       "Bagneux–Lucie Aubrac",  "IDFM:monomodalStopPlace:59626", &["M4"],         [-10.0, 0.0, 90.0]),
    // ═══════════════════════ Métro 5 ═════════════════════════════════════════
    ("m5_bobigny",       "Bobigny–Pablo Picasso", "IDFM:monomodalStopPlace:59630", &["M5"],         [ 60.0, 0.0,-120.0]),
    ("m5_eglise_pantin", "Église de Pantin",      "IDFM:monomodalStopPlace:59632", &["M5"],         [ 40.0, 0.0,-100.0]),
    ("m5_stalingrad",    "Stalingrad",            "IDFM:monomodalStopPlace:59634", &["M5"],         [ 12.0, 0.0, -82.0]),
    ("m5_gare_du_nord",  "Gare du Nord",          "IDFM:monomodalStopPlace:59635", &["M5"],         [ 12.0, 0.0, -80.0]),
    ("m5_gare_de_lest",  "Gare de l'Est",         "IDFM:monomodalStopPlace:59636", &["M5"],         [ 14.0, 0.0, -70.0]),
    ("m5_republique",    "République",            "IDFM:monomodalStopPlace:59638", &["M5"],         [ 10.0, 0.0, -30.0]),
    ("m5_bastille",      "Bastille",              "IDFM:monomodalStopPlace:59640", &["M5"],         [ 18.0, 0.0,   5.0]),
    ("m5_quai_rapee",    "Quai de la Rapée",      "IDFM:monomodalStopPlace:59641", &["M5"],         [ 22.0, 0.0,  12.0]),
    ("m5_austerlitz",    "Gare d'Austerlitz",     "IDFM:monomodalStopPlace:59642", &["M5"],         [ 12.0, 0.0,  18.0]),
    ("m5_place_italie",  "Place d'Italie",        "IDFM:monomodalStopPlace:59645", &["M5","M6","M7"],[ 10.0, 0.0, 40.0]),
    // ═══════════════════════ Métro 6 ═════════════════════════════════════════
    ("m6_etoile",        "Charles-de-Gaulle–Étoile","IDFM:monomodalStopPlace:59650",&["M6"],        [-30.0, 0.0,-28.0]),
    ("m6_trocadero",     "Trocadéro",             "IDFM:monomodalStopPlace:59652", &["M6","M9"],    [-28.0, 0.0,-18.0]),
    ("m6_passy",         "Passy",                 "IDFM:monomodalStopPlace:59653", &["M6"],         [-28.0, 0.0,-10.0]),
    ("m6_bir_hakeim",    "Bir-Hakeim",            "IDFM:monomodalStopPlace:59654", &["M6"],         [-25.0, 0.0, -2.0]),
    ("m6_cambronne",     "Cambronne",             "IDFM:monomodalStopPlace:59656", &["M6"],         [-18.0, 0.0, 12.0]),
    ("m6_montparnasse",  "Montparnasse–Bienvenüe","IDFM:monomodalStopPlace:59658", &["M6"],         [-12.0, 0.0, 30.0]),
    ("m6_denfert",       "Denfert-Rochereau",     "IDFM:monomodalStopPlace:59662", &["M6"],         [ -4.0, 0.0, 50.0]),
    ("m6_place_italie",  "Place d'Italie",        "IDFM:monomodalStopPlace:59665", &["M6"],         [ 10.0, 0.0, 40.0]),
    ("m6_bercy",         "Bercy",                 "IDFM:monomodalStopPlace:59668", &["M6","M14"],   [ 30.0, 0.0, 26.0]),
    ("m6_nation",        "Nation",                "IDFM:monomodalStopPlace:59670", &["M6"],         [ 45.0, 0.0, 10.0]),
    // ═══════════════════════ Métro 7 ═════════════════════════════════════════
    ("m7_la_courneuve",  "La Courneuve–8 Mai 1945","IDFM:monomodalStopPlace:59700",&["M7"],         [ 38.0, 0.0,-165.0]),
    ("m7_fort_aubervilliers","Fort d'Aubervilliers","IDFM:monomodalStopPlace:59702",&["M7"],        [ 30.0, 0.0,-140.0]),
    ("m7_stalingrad",    "Stalingrad",            "IDFM:monomodalStopPlace:59705", &["M7"],         [ 12.0, 0.0, -82.0]),
    ("m7_gare_de_lest",  "Gare de l'Est",         "IDFM:monomodalStopPlace:59706", &["M7"],         [ 14.0, 0.0, -70.0]),
    ("m7_opera",         "Opéra",                 "IDFM:monomodalStopPlace:59710", &["M7"],         [-12.0, 0.0,-20.0]),
    ("m7_palais_royal",  "Palais Royal",          "IDFM:monomodalStopPlace:59712", &["M7"],         [ -6.0, 0.0,-10.0]),
    ("m7_chatelet",      "Châtelet",              "IDFM:monomodalStopPlace:59714", &["M7"],         [  0.0, 0.0,  0.0]),
    ("m7_jussieu",       "Jussieu",               "IDFM:monomodalStopPlace:59718", &["M7","M10"],   [  5.0, 0.0, 20.0]),
    ("m7_place_italie",  "Place d'Italie",        "IDFM:monomodalStopPlace:59720", &["M7"],         [ 10.0, 0.0, 40.0]),
    ("m7_villejuif",     "Villejuif–Louis Aragon","IDFM:monomodalStopPlace:59730", &["M7"],         [ 10.0, 0.0, 90.0]),
    // ═══════════════════════ Métro 8 ═════════════════════════════════════════
    ("m8_balard",        "Balard",                "IDFM:monomodalStopPlace:59735", &["M8"],         [-35.0, 0.0,  8.0]),
    ("m8_la_motte",      "La Motte-Picquet",      "IDFM:monomodalStopPlace:59737", &["M8","M6","M10"],[-20.0, 0.0, 5.0]),
    ("m8_invalides",     "Invalides",             "IDFM:monomodalStopPlace:59740", &["M8","M13"],   [-18.0, 0.0,-15.0]),
    ("m8_concorde",      "Concorde",              "IDFM:monomodalStopPlace:59741", &["M8"],         [-15.0, 0.0,-18.0]),
    ("m8_madeleine",     "Madeleine",             "IDFM:monomodalStopPlace:59742", &["M8","M12","M14"],[-8.0, 0.0,-30.0]),
    ("m8_opera",         "Opéra",                 "IDFM:monomodalStopPlace:59743", &["M8"],         [-12.0, 0.0,-20.0]),
    ("m8_strasbourg",    "Strasbourg–Saint-Denis","IDFM:monomodalStopPlace:59746", &["M8"],         [  8.0, 0.0,-50.0]),
    ("m8_republique",    "République",            "IDFM:monomodalStopPlace:59748", &["M8"],         [ 10.0, 0.0,-30.0]),
    ("m8_bastille",      "Bastille",              "IDFM:monomodalStopPlace:59750", &["M8"],         [ 18.0, 0.0,  5.0]),
    ("m8_creteil",       "Créteil–Pointe du Lac", "IDFM:monomodalStopPlace:59770", &["M8"],         [ 60.0, 0.0, 80.0]),
    // ═══════════════════════ Métro 9 ═════════════════════════════════════════
    ("m9_pont_sevres",   "Pont de Sèvres",        "IDFM:monomodalStopPlace:59780", &["M9"],         [-55.0, 0.0, 15.0]),
    ("m9_trocadero",     "Trocadéro",             "IDFM:monomodalStopPlace:59785", &["M9"],         [-28.0, 0.0,-18.0]),
    ("m9_champs_elysees","Franklin D. Roosevelt", "IDFM:monomodalStopPlace:59790", &["M9","M1"],    [-18.0, 0.0,-22.0]),
    ("m9_st_lazare",     "Saint-Lazare",          "IDFM:monomodalStopPlace:59792", &["M9"],         [-15.0, 0.0,-45.0]),
    ("m9_strasbourg",    "Strasbourg–Saint-Denis","IDFM:monomodalStopPlace:59795", &["M9"],         [  8.0, 0.0,-50.0]),
    ("m9_republique",    "République",            "IDFM:monomodalStopPlace:59798", &["M9"],         [ 10.0, 0.0,-30.0]),
    ("m9_nation",        "Nation",                "IDFM:monomodalStopPlace:59800", &["M9"],         [ 45.0, 0.0, 10.0]),
    ("m9_mairie_montreuil","Mairie de Montreuil", "IDFM:monomodalStopPlace:59805", &["M9"],         [ 60.0, 0.0, -5.0]),
    // ═══════════════════════ Métro 10 ════════════════════════════════════════
    ("m10_boulogne",     "Boulogne–Pont de Saint-Cloud","IDFM:monomodalStopPlace:59810",&["M10"],   [-50.0, 0.0, 10.0]),
    ("m10_la_motte",     "La Motte-Picquet",      "IDFM:monomodalStopPlace:59815", &["M10"],        [-20.0, 0.0,  5.0]),
    ("m10_sevres_babylone","Sèvres-Babylone",     "IDFM:monomodalStopPlace:59818", &["M10","M12"],  [-10.0, 0.0, 15.0]),
    ("m10_odeon",        "Odéon",                 "IDFM:monomodalStopPlace:59820", &["M10"],        [ -5.0, 0.0, 15.0]),
    ("m10_jussieu",      "Jussieu",               "IDFM:monomodalStopPlace:59822", &["M10"],        [  5.0, 0.0, 20.0]),
    ("m10_austerlitz",   "Gare d'Austerlitz",     "IDFM:monomodalStopPlace:59825", &["M10"],        [ 12.0, 0.0, 18.0]),
    // ═══════════════════════ Métro 11 ════════════════════════════════════════
    ("m11_chatelet",     "Châtelet",              "IDFM:monomodalStopPlace:59830", &["M11"],        [  0.0, 0.0,  0.0]),
    ("m11_hotel_de_ville","Hôtel de Ville",       "IDFM:monomodalStopPlace:59831", &["M11"],        [  5.0, 0.0,  2.0]),
    ("m11_republique",   "République",            "IDFM:monomodalStopPlace:59833", &["M11"],        [ 10.0, 0.0,-30.0]),
    ("m11_belleville",   "Belleville",            "IDFM:monomodalStopPlace:59835", &["M11"],        [ 20.0, 0.0,-60.0]),
    ("m11_mairie_lilas", "Mairie des Lilas",      "IDFM:monomodalStopPlace:59840", &["M11"],        [ 40.0, 0.0,-60.0]),
    // ═══════════════════════ Métro 12 ════════════════════════════════════════
    ("m12_front_populaire","Front Populaire",     "IDFM:monomodalStopPlace:59845", &["M12"],        [ 10.0, 0.0,-110.0]),
    ("m12_marcadet",     "Marcadet–Poissonniers", "IDFM:monomodalStopPlace:59848", &["M12","M4"],   [  2.0, 0.0,-70.0]),
    ("m12_pigalle",      "Pigalle",               "IDFM:monomodalStopPlace:59850", &["M12"],        [ -4.0, 0.0,-60.0]),
    ("m12_st_lazare",    "Saint-Lazare",          "IDFM:monomodalStopPlace:59853", &["M12"],        [-15.0, 0.0,-45.0]),
    ("m12_madeleine",    "Madeleine",             "IDFM:monomodalStopPlace:59855", &["M12"],        [ -8.0, 0.0,-30.0]),
    ("m12_concorde",     "Concorde",              "IDFM:monomodalStopPlace:59856", &["M12"],        [-15.0, 0.0,-18.0]),
    ("m12_sevres_babylone","Sèvres-Babylone",     "IDFM:monomodalStopPlace:59858", &["M12"],        [-10.0, 0.0, 15.0]),
    ("m12_montparnasse", "Montparnasse–Bienvenüe","IDFM:monomodalStopPlace:59860", &["M12"],        [-12.0, 0.0, 30.0]),
    ("m12_mairie_issy",  "Mairie d'Issy",         "IDFM:monomodalStopPlace:59870", &["M12"],        [-35.0, 0.0, 40.0]),
    // ═══════════════════════ Métro 13 ════════════════════════════════════════
    ("m13_st_denis_u",   "Saint-Denis–Université","IDFM:monomodalStopPlace:43036", &["M13"],        [ 10.0, 0.0,-165.0]),
    ("m13_basilique",    "Basilique de St-Denis", "IDFM:monomodalStopPlace:59901", &["M13"],        [ 12.0, 0.0,-155.0]),
    ("m13_gabriel_peri", "Gabriel Péri",          "IDFM:monomodalStopPlace:59910", &["M13"],        [-14.0, 0.0,-155.0]),
    ("m13_la_fourche",   "La Fourche",            "IDFM:monomodalStopPlace:59915", &["M13"],        [ -8.0, 0.0,-62.0]),
    ("m13_place_clichy", "Place de Clichy",       "IDFM:monomodalStopPlace:59916", &["M13"],        [-10.0, 0.0,-55.0]),
    ("m13_st_lazare",    "Saint-Lazare",          "IDFM:monomodalStopPlace:59920", &["M13"],        [-15.0, 0.0,-45.0]),
    ("m13_champs_elysees","Champs-Élysées–Clemenceau","IDFM:monomodalStopPlace:59922",&["M13"],    [-18.0, 0.0,-22.0]),
    ("m13_invalides",    "Invalides",             "IDFM:monomodalStopPlace:59924", &["M13"],        [-18.0, 0.0,-15.0]),
    ("m13_montparnasse", "Montparnasse–Bienvenüe","IDFM:monomodalStopPlace:59926", &["M13"],        [-12.0, 0.0, 30.0]),
    ("m13_chatillon",    "Châtillon–Montrouge",   "IDFM:monomodalStopPlace:59930", &["M13"],        [-20.0, 0.0, 80.0]),
    // ═══════════════════════ Métro 14 ════════════════════════════════════════
    ("m14_st_denis_pleyel","Saint-Denis Pleyel",  "IDFM:monomodalStopPlace:59940", &["M14"],        [ 15.0, 0.0,-145.0]),
    ("m14_mairie_st_ouen","Mairie de Saint-Ouen", "IDFM:monomodalStopPlace:59648", &["M14"],        [ -5.0, 0.0,-120.0]),
    ("m14_st_lazare",    "Saint-Lazare",          "IDFM:monomodalStopPlace:59633", &["M14"],        [-15.0, 0.0, -45.0]),
    ("m14_madeleine",    "Madeleine",             "IDFM:monomodalStopPlace:59634", &["M14"],        [ -8.0, 0.0, -30.0]),
    ("m14_pyramides",    "Pyramides",             "IDFM:monomodalStopPlace:59635", &["M14"],        [ -5.0, 0.0, -18.0]),
    ("m14_chatelet",     "Châtelet",              "IDFM:monomodalStopPlace:59636", &["M14"],        [  0.0, 0.0,   0.0]),
    ("m14_gare_de_lyon", "Gare de Lyon",          "IDFM:monomodalStopPlace:59650", &["M14"],        [ 30.0, 0.0,  15.0]),
    ("m14_bercy",        "Bercy",                 "IDFM:monomodalStopPlace:59651", &["M14"],        [ 30.0, 0.0,  26.0]),
    ("m14_cour_st_emilion","Cour Saint-Émilion",  "IDFM:monomodalStopPlace:59652", &["M14"],        [ 25.0, 0.0,  40.0]),
    ("m14_bibliotheque", "Bibliothèque F.Mitterrand","IDFM:monomodalStopPlace:59653",&["M14"],      [ 15.0, 0.0,  40.0]),
    ("m14_olympiades",   "Olympiades",            "IDFM:monomodalStopPlace:59654", &["M14"],        [ 25.0, 0.0,  55.0]),
    ("m14_aeroport_orly","Aéroport d'Orly",       "IDFM:monomodalStopPlace:59660", &["M14"],        [ 10.0, 0.0, 150.0]),
];

// ─────────────────────────────────────────────────────────────────────────────
// Scene spawn
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Scale and environment constants
// ─────────────────────────────────────────────────────────────────────────────

/// Scale factor: 1 unit in IDF_STATIONS pos → this many Bevy world units.
/// At 300: 1 station-coord unit ≈ 75 m real distance.
const STATION_SCALE: f32 = 300.0;

/// World-unit radius of inner Paris (périphérique boundary).
const PARIS_INNER_RADIUS: f32 = 15_000.0;

/// Station proximity radius: departure board appears within this distance.
const PROXIMITY_RADIUS: f32 = 6_000.0;

/// Trains start shooting when player is within this world-unit range.
const TRAIN_SHOOT_RANGE: f32 = 40_000.0;

/// Train labels are visible up to this range.
const TRAIN_LABEL_RANGE: f32 = 12_000.0;

fn station_world_pos(raw: [f32; 3]) -> Vec3 {
    Vec3::new(raw[0] * STATION_SCALE, raw[1] * STATION_SCALE, raw[2] * STATION_SCALE)
}

/// Position of a station with the per-line elevation offset added.
fn station_world_pos_elevated(raw: [f32; 3], line_id: &str) -> Vec3 {
    let y_off = IDF_LINES.iter().find(|l| l.0 == line_id)
        .map(|l| l.3).unwrap_or(0.0);
    Vec3::new(raw[0] * STATION_SCALE, raw[1] * STATION_SCALE + y_off, raw[2] * STATION_SCALE)
}

/// Helper: return the display color for a line id.
fn line_color(line_id: &str) -> [f32; 3] {
    IDF_LINES.iter().find(|l| l.0 == line_id)
        .map(|l| l.2).unwrap_or([0.5, 0.5, 0.5])
}

/// Spawns the IDF transport network map.
/// Returns the player spawn transform (looking over Paris from above).
pub fn spawn_idf_transport_scene(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    idf_config: &Res<IdfConfig>,
) -> Transform {
    let all_indices: Vec<usize> = (0..IDF_STATIONS.len()).collect();
    let selected: &[usize] = if idf_config.selected_stations.is_empty() {
        &all_indices
    } else {
        &idf_config.selected_stations
    };

    let mut rng = rand::thread_rng();

    // ── Parisian overcast lighting ────────────────────────────────────────────
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::rgb(0.88, 0.90, 0.95),
                illuminance: 5_500.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_rotation(
                Quat::from_euler(EulerRot::YXZ, 0.25, -0.55, 0.0)
            ),
            ..default()
        },
        SceneEntity,
    ));
    // Soft ambient fill — grey-blue diffuse typical of Paris overcast
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                color: Color::rgb(0.50, 0.54, 0.62),
                intensity: 1_200_000.0,
                range: 300_000.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 20_000.0, 0.0),
            ..default()
        },
        SceneEntity,
    ));

    // ── Sky dome (Parisian grey overcast) ─────────────────────────────────────
    let sky_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.68, 0.72, 0.80),
        emissive:   Color::rgb(0.38, 0.42, 0.50),
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 900_000.0,
                sectors: 24,
                stacks: 12,
            })),
            material: sky_mat,
            ..default()
        },
        SceneEntity,
    ));

    // ── Cloud patches (flat white boxes at altitude) ──────────────────────────
    let cloud_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(0.94, 0.95, 0.97, 0.92),
        emissive:   Color::rgb(0.22, 0.24, 0.28),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for _ in 0..45 {
        let cx = rng.gen_range(-80_000.0_f32..80_000.0);
        let cz = rng.gen_range(-100_000.0_f32..100_000.0);
        let cy = rng.gen_range(9_000.0_f32..14_000.0);
        let cw = rng.gen_range(4_000.0_f32..11_000.0);
        let cd = rng.gen_range(2_500.0_f32..7_000.0);
        let ch = rng.gen_range(300.0_f32..700.0);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box {
                    min_x: -cw * 0.5, max_x: cw * 0.5,
                    min_y: 0.0,       max_y: ch,
                    min_z: -cd * 0.5, max_z: cd * 0.5,
                })),
                material: cloud_mat.clone(),
                transform: Transform::from_xyz(cx, cy, cz),
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Outer ground (green suburban / Île-de-France countryside) ─────────────
    let outer_ground_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.27, 0.52, 0.20),
        metallic: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 1_400_000.0, subdivisions: 0 })),
            material: outer_ground_mat,
            transform: Transform::from_xyz(0.0, -105.0, 0.0),
            ..default()
        },
        SceneEntity,
    ));

    // ── Inner Paris ground (Haussmann beige / city stone) ─────────────────────
    let inner_paris_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.84, 0.79, 0.65),
        metallic: 0.0,
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane {
                size: PARIS_INNER_RADIUS * 2.3,
                subdivisions: 0,
            })),
            material: inner_paris_mat,
            transform: Transform::from_xyz(0.0, -100.0, -2_000.0), // shifted slightly N
            ..default()
        },
        SceneEntity,
    ));

    // ── Seine river ───────────────────────────────────────────────────────────
    // Three segments approximating the Seine's path through central Paris.
    // Coordinates at STATION_SCALE = 300: positive z = south.
    let seine_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.25, 0.50, 0.75),
        emissive:   Color::rgb(0.04, 0.08, 0.20),
        metallic: 0.3,
        perceptual_roughness: 0.35,
        ..default()
    });
    // (x1, z1, x2, z2) endpoint pairs
    let seine_segments: &[(f32, f32, f32, f32)] = &[
        (-16_000.0,  1_500.0,  -3_000.0,   600.0),  // rive gauche west
        ( -3_000.0,    600.0,   3_000.0,   300.0),  // île de la Cité
        (  3_000.0,    300.0,  18_000.0,  1_800.0), // rive droite east
    ];
    let seine_width = 280.0_f32;
    for &(x1, z1, x2, z2) in seine_segments {
        let a = Vec3::new(x1, -92.0, z1);
        let b = Vec3::new(x2, -92.0, z2);
        let dir = b - a;
        let len = dir.length();
        let mid = (a + b) * 0.5;
        let angle = f32::atan2(dir.x, dir.z);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box {
                    min_x: -seine_width * 0.5, max_x: seine_width * 0.5,
                    min_y: 0.0,                max_y: 8.0,
                    min_z: -len * 0.5,         max_z: len * 0.5,
                })),
                material: seine_mat.clone(),
                transform: Transform {
                    translation: mid,
                    rotation: Quat::from_rotation_y(angle),
                    ..default()
                },
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Tour Eiffel (geometric landmark) ─────────────────────────────────────
    // Positioned between Bir-Hakeim and Trocadéro (west Paris, near 7e arr.)
    {
        let base_x = -7_800.0_f32;
        let base_z = -2_400.0_f32;
        let base_y = -95.0_f32;
        let iron_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(0.22, 0.20, 0.18),
            emissive:   Color::rgb(0.04, 0.04, 0.03),
            metallic: 0.9,
            perceptual_roughness: 0.5,
            ..default()
        });
        // 4 diagonal legs (tapered boxes converging upward)
        let leg_offsets = [(-550.0_f32, -550.0_f32), (550.0, -550.0), (-550.0, 550.0), (550.0, 550.0)];
        for (ox, oz) in leg_offsets {
            // Leg: thick at base, position so it leans inward at higher y is
            // simulated by scaling; use two sections per leg
            commands.spawn((PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box {
                    min_x: -80.0, max_x: 80.0,
                    min_y: 0.0,   max_y: 1_200.0,
                    min_z: -80.0, max_z: 80.0,
                })),
                material: iron_mat.clone(),
                transform: Transform {
                    translation: Vec3::new(base_x + ox * 0.6, base_y, base_z + oz * 0.6),
                    rotation: Quat::from_euler(
                        EulerRot::YXZ,
                        f32::atan2(-oz, -ox),
                        0.35,
                        0.0,
                    ),
                    ..default()
                },
                ..default()
            }, SceneEntity));
        }
        // First platform (400 m level)
        commands.spawn((PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box {
                min_x: -350.0, max_x: 350.0,
                min_y: 0.0,    max_y: 120.0,
                min_z: -350.0, max_z: 350.0,
            })),
            material: iron_mat.clone(),
            transform: Transform::from_xyz(base_x, base_y + 1_050.0, base_z),
            ..default()
        }, SceneEntity));
        // Upper shaft
        commands.spawn((PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box {
                min_x: -120.0, max_x: 120.0,
                min_y: 0.0,    max_y: 2_000.0,
                min_z: -120.0, max_z: 120.0,
            })),
            material: iron_mat.clone(),
            transform: Transform::from_xyz(base_x, base_y + 1_170.0, base_z),
            ..default()
        }, SceneEntity));
        // Spire
        commands.spawn((PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cylinder {
                radius: 25.0,
                height: 800.0,
                resolution: 8,
                segments: 1,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.80, 0.75, 0.50),
                emissive: Color::rgb(0.60, 0.50, 0.10),
                metallic: 0.8,
                perceptual_roughness: 0.2,
                ..default()
            }),
            transform: Transform::from_xyz(base_x, base_y + 3_170.0, base_z),
            ..default()
        }, SceneEntity));
    }

    // ── Arc de Triomphe (geometric landmark, at Étoile) ───────────────────────
    {
        let arc_x = -9_000.0_f32;
        let arc_z = -8_400.0_f32;
        let arc_y = -95.0_f32;
        let stone_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(0.82, 0.76, 0.60),
            emissive:   Color::rgb(0.08, 0.07, 0.04),
            metallic: 0.1,
            perceptual_roughness: 0.8,
            ..default()
        });
        // Left pillar
        commands.spawn((PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box {
                min_x: -200.0, max_x:  200.0,
                min_y: 0.0,    max_y: 1_000.0,
                min_z: -250.0, max_z:  250.0,
            })),
            material: stone_mat.clone(),
            transform: Transform::from_xyz(arc_x - 350.0, arc_y, arc_z),
            ..default()
        }, SceneEntity));
        // Right pillar
        commands.spawn((PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box {
                min_x: -200.0, max_x:  200.0,
                min_y: 0.0,    max_y: 1_000.0,
                min_z: -250.0, max_z:  250.0,
            })),
            material: stone_mat.clone(),
            transform: Transform::from_xyz(arc_x + 350.0, arc_y, arc_z),
            ..default()
        }, SceneEntity));
        // Arch horizontal beam (top)
        commands.spawn((PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Box {
                min_x: -750.0, max_x:  750.0,
                min_y: 0.0,    max_y:  280.0,
                min_z: -250.0, max_z:  250.0,
            })),
            material: stone_mat.clone(),
            transform: Transform::from_xyz(arc_x, arc_y + 820.0, arc_z),
            ..default()
        }, SceneEntity));
    }

    // ── Hills scattered around the Île-de-France ──────────────────────────────
    // (x, z, base_horizontal_radius, height) – all in world units.
    // Placed outside the Paris ring road, beyond the beige inner zone.
    let hill_defs: &[(f32, f32, f32, f32)] = &[
        (-42_000.0, -52_000.0, 5_500.0, 2_200.0), // Montmorency forest ridge
        ( 18_000.0, -58_000.0, 4_800.0, 1_800.0), // plains north of Paris
        ( 52_000.0, -78_000.0, 6_200.0, 2_800.0), // hills toward CDG
        ( 68_000.0,  18_000.0, 5_000.0, 1_600.0), // Val-de-Marne plateau
        ( 38_000.0,  72_000.0, 5_500.0, 2_000.0), // Essonne hills
        (-12_000.0,  78_000.0, 4_600.0, 1_500.0), // Massy/Saclay plateau
        (-58_000.0,  42_000.0, 7_000.0, 2_600.0), // Versailles ridge
        (-58_000.0, -12_000.0, 5_800.0, 2_000.0), // Hauts-de-Seine hills
        (-32_000.0, -62_000.0, 4_500.0, 1_800.0), // Argenteuil butte
        ( 82_000.0,-118_000.0, 6_000.0, 2_400.0), // butte near Roissy
    ];
    let hill_mat = materials.add(StandardMaterial {
        base_color: Color::rgb(0.26, 0.52, 0.18),
        metallic: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    let hill_mesh = meshes.add(Mesh::from(shape::UVSphere { radius: 1.0, sectors: 16, stacks: 8 }));

    // Build the IDF terrain kill-data while spawning hills
    let mut terrain_data = IdfTerrainData {
        floor_y: -80.0, // player dies below this Y (ground surface ≈ -95)
        kill_zones: Vec::new(),
    };
    // Monuments
    terrain_data.kill_zones.push((Vec3::new(-7_800.0, 1_200.0, -2_400.0), 1_000.0, 1_400.0)); // Tour Eiffel
    terrain_data.kill_zones.push((Vec3::new(-9_000.0,   300.0, -8_400.0),   900.0,   500.0)); // Arc de Triomphe

    for &(hx, hz, hr, height) in hill_defs {
        let gy = -95.0_f32;
        commands.spawn((
            PbrBundle {
                mesh: hill_mesh.clone(),
                material: hill_mat.clone(),
                transform: Transform {
                    translation: Vec3::new(hx, gy, hz),
                    scale: Vec3::new(hr, height, hr),
                    ..default()
                },
                ..default()
            },
            SceneEntity,
        ));
        // Kill zone: centred at 50 % height above ground base
        terrain_data.kill_zones.push((Vec3::new(hx, gy + height * 0.5, hz), hr * 0.85, height * 0.6));
    }
    commands.insert_resource(terrain_data);

    // ── Subtle grid lines on the ground for geographic context ────────────────
    let grid_mat = materials.add(StandardMaterial {
        base_color: Color::rgba(0.40, 0.35, 0.25, 0.4),
        metallic: 0.0,
        perceptual_roughness: 1.0,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for i in -6..=6 {
        let pos = i as f32 * 12_000.0;
        // east-west grid line
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box {
                    min_x: -80_000.0, max_x: 80_000.0,
                    min_y: -1.0, max_y: 1.0,
                    min_z: -4.0, max_z: 4.0,
                })),
                material: grid_mat.clone(),
                transform: Transform::from_xyz(0.0, -94.0, pos),
                ..default()
            },
            SceneEntity,
        ));
        // north-south grid line
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box {
                    min_x: -4.0, max_x: 4.0,
                    min_y: -1.0, max_y: 1.0,
                    min_z: -80_000.0, max_z: 80_000.0,
                })),
                material: grid_mat.clone(),
                transform: Transform::from_xyz(pos, -94.0, 0.0),
                ..default()
            },
            SceneEntity,
        ));
    }

    // ── Line rails (elevated per line) ────────────────────────────────────────
    let rail_cylinder_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 18.0,
        height: 1.0, // scaled at spawn
        resolution: 6,
        segments: 1,
    }));

    for &(line_id, _label, color, _y_off) in IDF_LINES {
        let rail_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(color[0] * 0.6, color[1] * 0.6, color[2] * 0.6),
            emissive: Color::rgb(color[0] * 0.4, color[1] * 0.4, color[2] * 0.4),
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        });

        // Collect stations for this line in catalogue order
        let stations_on_line: Vec<Vec3> = IDF_STATIONS.iter()
            .filter(|s| s.3.contains(&line_id))
            .map(|s| station_world_pos_elevated(s.4, line_id))
            .collect();

        // Draw segments between consecutive stations
        for pair in stations_on_line.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let mid = (a + b) * 0.5;
            let dir = b - a;
            let len = dir.length();
            if len < 0.1 { continue; }
            let rot = Quat::from_rotation_arc(Vec3::Y, dir.normalize());
            commands.spawn((
                PbrBundle {
                    mesh: rail_cylinder_mesh.clone(),
                    material: rail_mat.clone(),
                    transform: Transform {
                        translation: mid,
                        rotation: rot,
                        scale: Vec3::new(1.0, len, 1.0),
                    },
                    ..default()
                },
                SceneEntity,
            ));
        }

        // Vertical support pylons from ground to the rail for style
        let pylon_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(color[0] * 0.2, color[1] * 0.2, color[2] * 0.2),
            metallic: 0.9,
            perceptual_roughness: 0.4,
            ..default()
        });
        // Place pylons at every 3rd station on this line to avoid clutter
        for (i, s) in IDF_STATIONS.iter().filter(|s| s.3.contains(&line_id)).enumerate() {
            if i % 3 != 0 { continue; }
            let wp = station_world_pos_elevated(s.4, line_id);
            let ground_y = -90.0;
            let height = wp.y - ground_y;
            if height < 20.0 { continue; }
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box {
                        min_x: -8.0, max_x: 8.0,
                        min_y: 0.0, max_y: height,
                        min_z: -8.0, max_z: 8.0,
                    })),
                    material: pylon_mat.clone(),
                    transform: Transform::from_xyz(wp.x, ground_y, wp.z),
                    ..default()
                },
                SceneEntity,
            ));
        }
    }

    // ── Station spheres ───────────────────────────────────────────────────────
    let station_mesh = meshes.add(Mesh::from(shape::UVSphere {
        radius: 1.0,
        sectors: 12,
        stacks: 8,
    }));

    for (idx, &(id, label, _prim_id, lines, raw_pos)) in IDF_STATIONS.iter().enumerate() {
        // Only show RER B stations
        if !lines.contains(&"RER_B") { continue; }
        let world_pos = station_world_pos(raw_pos);
        let is_selected = selected.contains(&idx);

        // Pick colour from first line
        let c = line_color(lines.first().copied().unwrap_or(""));
        let base_color = Color::rgb(c[0], c[1], c[2]);

        let emissive = Color::rgb(c[0] * 0.6, c[1] * 0.6, c[2] * 0.6);

        // Hub stations (≥3 lines) get a bigger sphere
        let is_hub = lines.len() >= 3;
        let radius = if is_hub { 220.0 } else if is_selected { 170.0 } else { 110.0 };

        // Selected stations glow golden-white
        let (final_base, final_emissive) = if is_selected {
            (
                Color::rgb(1.00, 0.88, 0.30),
                Color::rgb(3.50, 2.80, 0.40),
            )
        } else {
            (base_color, emissive)
        };

        let station_mat_final = materials.add(StandardMaterial {
            base_color: final_base,
            emissive: final_emissive,
            metallic: if is_selected { 0.2 } else { 0.5 },
            perceptual_roughness: if is_selected { 0.3 } else { 0.2 },
            ..default()
        });

        commands.spawn((
            PbrBundle {
                mesh: station_mesh.clone(),
                material: station_mat_final,
                transform: Transform::from_translation(world_pos)
                    .with_scale(Vec3::splat(radius)),
                ..default()
            },
            IdfStation { station_idx: idx },
            SceneEntity,
        ));

        // Extra glow ring for selected stations
        if is_selected {
            let ring_mat = materials.add(StandardMaterial {
                base_color: Color::rgba(1.0, 0.85, 0.20, 0.6),
                emissive: Color::rgb(4.0, 3.0, 0.30),
                alpha_mode: AlphaMode::Add,
                unlit: true,
                ..default()
            });
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::UVSphere {
                        radius: 1.0,
                        sectors: 12,
                        stacks: 8,
                    })),
                    material: ring_mat,
                    transform: Transform::from_translation(world_pos)
                        .with_scale(Vec3::splat(radius * 1.55)),
                    ..default()
                },
                SceneEntity,
            ));
        }

        let _ = (id, label);
    }

    // ── Enemy trains ──────────────────────────────────────────────────────────
    spawn_enemy_trains(commands, meshes, materials, selected);

    // ── Player spawn: elevated above the RER B line ─────────────────────────
    // RER B spans z≈-96 000 (CDG) to z≈+78 000 (St-Rémy); x center ≈ 9 000.
    // Position the camera south-of-center and high to see the full line.
    Transform::from_xyz(9_000.0, 85_000.0, 45_000.0)
        .looking_at(Vec3::new(3_000.0, 0.0, -15_000.0), Vec3::Y)
}

fn spawn_enemy_trains(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    _selected_stations: &[usize],
) {
    for &(line_id, label, color, _y_off) in IDF_LINES {
        // Always use all stations on this line as waypoints (no selection filter)
        let waypoints: Vec<Vec3> = IDF_STATIONS.iter()
            .filter(|s| s.3.contains(&line_id))
            .map(|s| station_world_pos(s.4))
            .collect();

        if waypoints.len() < 2 { continue; }

        let train_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(color[0], color[1], color[2]),
            emissive: Color::rgb(color[0] * 1.8, color[1] * 1.8, color[2] * 1.8),
            metallic: 0.85,
            perceptual_roughness: 0.15,
            ..default()
        });

        // Bright emissive "destination sign" on the front/top of each train
        let sign_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(color[0] * 0.4, color[1] * 0.4, color[2] * 0.4),
            emissive: Color::rgb(color[0] * 5.5, color[1] * 5.5, color[2] * 5.5),
            unlit: true,
            ..default()
        });

        // Undercarriage shadow panel (dark, metallic)
        let chassis_mat = materials.add(StandardMaterial {
            base_color: Color::rgb(0.08, 0.08, 0.10),
            metallic: 0.95,
            perceptual_roughness: 0.4,
            ..default()
        });

        // RER trains are longer / faster than Métro trains
        let is_rer = line_id.starts_with("RER");
        let speed = if is_rer { 2_800.0 } else { 1_800.0 };
        let train_len  = if is_rer { 600.0 } else { 360.0 };
        let train_half_w = if is_rer { 75.0 } else { 60.0 };
        let train_half_h = if is_rer { 55.0 } else { 45.0 };
        let shoot_interval: f32 = if is_rer { 3.5 } else { 5.0 };

        let mut rng = rand::thread_rng();

        // Spawn 5 trains per line, evenly spread along the route
        for offset in 0..5usize {
            let start_wp = (offset * waypoints.len() / 2).min(waypoints.len() - 1);
            let start_pos = waypoints[start_wp];

            // Main carriage body
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box {
                        min_x: -train_half_w,    max_x: train_half_w,
                        min_y: -train_half_h,    max_y: train_half_h,
                        min_z: -train_len / 2.0, max_z: train_len / 2.0,
                    })),
                    material: train_mat.clone(),
                    transform: Transform::from_translation(start_pos),
                    ..default()
                },
                IdfTrain {
                    line_id,
                    waypoints: waypoints.clone(),
                    current_wp: (start_wp + 1) % waypoints.len(),
                    speed,
                    shoot_timer: rng.gen_range(0.0..shoot_interval),
                    shoot_interval,
                },
                SceneEntity,
            ))
            .with_children(|parent| {
                // Emissive line-sign panel on top of the train (very bright)
                parent.spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box {
                        min_x: -(train_half_w - 5.0), max_x: train_half_w - 5.0,
                        min_y: 0.0,                   max_y: 18.0,
                        min_z: -(train_len * 0.55),   max_z: train_len * 0.55,
                    })),
                    material: sign_mat.clone(),
                    transform: Transform::from_xyz(0.0, train_half_h, 0.0),
                    ..default()
                });
                // Dark chassis underside
                parent.spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box {
                        min_x: -(train_half_w + 5.0), max_x: train_half_w + 5.0,
                        min_y: -8.0, max_y: 0.0,
                        min_z: -(train_len * 0.48), max_z: train_len * 0.48,
                    })),
                    material: chassis_mat.clone(),
                    transform: Transform::from_xyz(0.0, -train_half_h, 0.0),
                    ..default()
                });
                // Front headlamp disc
                parent.spawn(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cylinder {
                        radius: 20.0,
                        height: 15.0,
                        resolution: 10,
                        segments: 1,
                    })),
                    material: materials.add(StandardMaterial {
                        base_color: Color::rgb(1.0, 0.95, 0.80),
                        emissive: Color::rgb(8.0, 7.5, 5.0),
                        unlit: true,
                        ..default()
                    }),
                    transform: Transform {
                        translation: Vec3::new(0.0, 0.0, train_len / 2.0 + 5.0),
                        rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                        ..default()
                    },
                    ..default()
                });
            });
        }
        let _ = label;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime systems
// ─────────────────────────────────────────────────────────────────────────────

/// Moves IDF enemy trains along their waypoints each frame.
pub fn idf_train_movement_system(
    mut train_q: Query<(&mut Transform, &mut IdfTrain)>,
    time: Res<Time>,
    paused: Res<TimePaused>,
    chat: Res<crate::systems::ui::copilot_chat::LlmChatState>,
) {
    if paused.0 || chat.open { return; }
    let dt = time.delta_seconds();

    for (mut transform, mut train) in &mut train_q {
        if train.waypoints.is_empty() { continue; }
        let target = train.waypoints[train.current_wp];
        let dir = target - transform.translation;
        let dist = dir.length();

        if dist < train.speed * dt * 1.5 {
            // Arrived at waypoint → advance
            train.current_wp = (train.current_wp + 1) % train.waypoints.len();
        } else {
            transform.translation += dir.normalize() * train.speed * dt;
            // Orient along travel direction (look_at needs a non-degenerate up)
            if (target - transform.translation).length() > 1.0 {
                transform.look_at(target, Vec3::Y);
            }
        }
    }
}

/// IDF trains periodically shoot homing missiles at the player.
pub fn idf_train_shoot_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut train_q: Query<(&Transform, &mut IdfTrain)>,
    camera_q: Query<&Transform, With<MainCamera>>,
    time: Res<Time>,
    paused: Res<TimePaused>,
    chat: Res<crate::systems::ui::copilot_chat::LlmChatState>,
    game_timer: Res<GameTimer>,
) {
    // Trains start shooting after 8 seconds
    if paused.0 || chat.open || game_timer.0 < 8.0 { return; }
    let dt = time.delta_seconds();
    let Ok(cam) = camera_q.get_single() else { return };
    let player_pos = cam.translation;

    for (train_tf, mut train) in &mut train_q {
        train.shoot_timer -= dt;
        if train.shoot_timer > 0.0 { continue; }

        // Reset timer
        train.shoot_timer = train.shoot_interval;

        let dist = (train_tf.translation - player_pos).length();
        if dist > TRAIN_SHOOT_RANGE || dist < 800.0 { continue; }

        // Spawn a line-coloured homing missile from the train's front
        let c = line_color(train.line_id);
        let toward_player = (player_pos - train_tf.translation).normalize_or_zero();
        let missile_rot = Quat::from_rotation_arc(Vec3::Y, toward_player);
        let spawn_pos = train_tf.translation + toward_player * 200.0;

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cylinder {
                    radius: 20.0,
                    height: 160.0,
                    resolution: 10,
                    segments: 1,
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgb(c[0], c[1], c[2]),
                    emissive: Color::rgb(c[0] * 4.0, c[1] * 4.0, c[2] * 4.0),
                    perceptual_roughness: 0.4,
                    metallic: 0.7,
                    ..default()
                }),
                transform: Transform::from_translation(spawn_pos).with_rotation(missile_rot),
                ..default()
            },
            Missile { speed: 18_000.0, turn_rate: 1.6, lifetime: 18.0 },
        ));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Screen-space label systems
// ─────────────────────────────────────────────────────────────────────────────

/// Spawns a UI text label for each new IdfTrain entity.
/// Runs in Update so it reacts to actors spawned in any frame.
pub fn idf_on_train_added_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    added_trains: Query<(Entity, &IdfTrain), Added<IdfTrain>>,
) {
    let font = asset_server.load(crate::setup::resolve_ui_font_path());
    for (entity, train) in &added_trains {
        // Display the line id as short uppercase text (M1, RER A, etc.)
        let display = train.line_id.replace("RER_", "RER ").replace("_", " ");
        let c = line_color(train.line_id);
        commands.spawn((
            TextBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top:  Val::Px(0.0),
                    ..default()
                },
                text: Text::from_section(
                    display,
                    TextStyle {
                        font: font.clone(),
                        font_size: 16.0,
                        color: Color::rgb(c[0], c[1], c[2]),
                    },
                ),
                visibility: Visibility::Hidden,
                background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.55)),
                ..default()
            },
            IdfTrainLabel { target: entity },
            SceneEntity,
        ));
    }
}

/// Each frame projects each train to screen space and updates its label position.
pub fn idf_update_train_labels_system(
    mut label_q: Query<(&mut Style, &mut Visibility, &IdfTrainLabel)>,
    train_q: Query<&Transform, With<IdfTrain>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    arm: Res<CameraArmOffset>,
) {
    let Ok((camera, cam_gt)) = camera_q.get_single() else { return };
    let player_pos = cam_gt.translation() - arm.0;

    for (mut style, mut vis, label) in &mut label_q {
        let Ok(train_tf) = train_q.get(label.target) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let train_pos = train_tf.translation;
        let dist = (train_pos - player_pos).length();

        if dist > TRAIN_LABEL_RANGE {
            *vis = Visibility::Hidden;
            continue;
        }

        // Project a point above the train to screen space
        let above = train_pos + Vec3::Y * 180.0;
        if let Some(screen) = camera.world_to_viewport(cam_gt, above) {
            *vis = Visibility::Visible;
            style.left = Val::Px(screen.x - 18.0);
            style.top  = Val::Px(screen.y - 10.0);
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

/// Updates the proximity departure popup above the nearest selected station.
pub fn idf_update_proximity_popup_system(
    mut popup_q: Query<(&mut Style, &mut Text, &mut Visibility), With<IdfProximityPopup>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    station_q: Query<(&Transform, &IdfStation)>,
    next_trains: Res<IdfNextTrains>,
    idf_config: Res<IdfConfig>,
    arm: Res<CameraArmOffset>,
) {
    let Ok((camera, cam_gt)) = camera_q.get_single() else { return };
    let player_pos = cam_gt.translation() - arm.0;

    let nearest = station_q.iter()
        .filter(|(_, s)| idf_config.selected_stations.contains(&s.station_idx))
        .min_by(|(a, _), (b, _)| {
            let da = (a.translation - player_pos).length();
            let db = (b.translation - player_pos).length();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

    let Ok((mut style, mut text, mut vis)) = popup_q.get_single_mut() else { return };

    if let Some((st_tf, st_data)) = nearest {
        let dist = (st_tf.translation - player_pos).length();
        if dist > PROXIMITY_RADIUS {
            *vis = Visibility::Hidden;
            return;
        }

        let station = IDF_STATIONS.get(st_data.station_idx);
        let station_name = station.map(|s| s.1).unwrap_or("?");
        let prim_id = station.map(|s| s.2).unwrap_or("");
        let deps = next_trains.departures.get(prim_id)
            .cloned()
            .unwrap_or_else(|| make_demo_data(
                IDF_STATIONS.get(st_data.station_idx).map(|s| s.0).unwrap_or("")
            ));

        // Show above the station sphere
        let above = st_tf.translation + Vec3::Y * 700.0;
        if let Some(screen) = camera.world_to_viewport(cam_gt, above) {
            *vis = Visibility::Visible;
            style.left = Val::Px((screen.x - 160.0).max(4.0));
            style.top  = Val::Px((screen.y - 10.0).max(4.0));

            let mut content = format!("📍 {station_name}\n");
            for d in deps.iter().take(8) {
                content.push_str(&format!("  {d}\n"));
            }
            text.sections[0].value = content;
        } else {
            *vis = Visibility::Hidden;
        }
    } else {
        *vis = Visibility::Hidden;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Real-time next-train data
// ─────────────────────────────────────────────────────────────────────────────

const PRIM_BASE: &str = "https://prim.iledefrance-mobilites.fr/marketplace/stop-monitoring";
const PRIM_POLL_SECONDS: f32 = 30.0;

/// Timer resource for PRIM API polling.
#[derive(Resource)]
pub struct IdfPrimPollTimer(pub Timer);

impl Default for IdfPrimPollTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(PRIM_POLL_SECONDS, TimerMode::Repeating))
    }
}

/// Periodically fires background HTTP requests to the PRIM API for each
/// selected station and writes results into `IdfNextTrains`.
pub fn idf_fetch_next_trains_system(
    mut timer: ResMut<IdfPrimPollTimer>,
    mut next_trains: ResMut<IdfNextTrains>,
    idf_config: Res<IdfConfig>,
    time: Res<Time>,
) {
    // Poll any pending result first
    if let Some(slot) = &next_trains.pending {
        let taken = slot.lock().ok().and_then(|mut g| g.take());
        if let Some(map) = taken {
            next_trains.departures.extend(map);
            next_trains.pending = None;
        }
    }

    timer.0.tick(time.delta());
    if !timer.0.just_finished() { return; }
    if next_trains.pending.is_some() { return; }

    // Collect prim_ids for selected stations
    let prim_ids: Vec<(&'static str, &'static str)> = idf_config.selected_stations.iter()
        .filter_map(|&i| IDF_STATIONS.get(i).map(|s| (s.0, s.2)))
        .collect();

    if prim_ids.is_empty() { return; }

    let slot: Arc<Mutex<Option<std::collections::HashMap<String, Vec<String>>>>> =
        Arc::new(Mutex::new(None));
    let slot_clone = Arc::clone(&slot);
    next_trains.pending = Some(slot);

    // We read the API key from an env var PRIM_API_KEY; if unset, use demo data
    let api_key = std::env::var("PRIM_API_KEY").unwrap_or_default();

    std::thread::spawn(move || {
        let mut all: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        for (station_id, prim_id) in prim_ids {
            if api_key.is_empty() {
                // Demo mode: generate plausible fake data
                let demo = make_demo_data(station_id);
                all.insert(prim_id.to_owned(), demo);
                continue;
            }

            let url = format!("{}?MonitoringRef={}", PRIM_BASE, prim_id);
            let result = ureq::get(&url)
                .set("apikey", &api_key)
                .call();

            let departures = match result {
                Err(_) => make_demo_data(station_id),
                Ok(resp) => match resp.into_string() {
                    Err(_) => make_demo_data(station_id),
                    Ok(raw) => parse_prim_response(&raw),
                },
            };
            all.insert(prim_id.to_owned(), departures);
        }

        if let Ok(mut guard) = slot_clone.lock() {
            *guard = Some(all);
        }
    });
}

fn parse_prim_response(raw: &str) -> Vec<String> {
    let mut result = Vec::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        let calls = &v["Siri"]["ServiceDelivery"]["StopMonitoringDelivery"][0]["MonitoredStopVisit"];
        if let Some(arr) = calls.as_array() {
            for call in arr.iter().take(6) {
                let line = call["MonitoredVehicleJourney"]["PublishedLineName"]["_value"]
                    .as_str().unwrap_or("?");
                let dest = call["MonitoredVehicleJourney"]["DestinationName"][0]["_value"]
                    .as_str().unwrap_or("?");
                let exp = call["MonitoredVehicleJourney"]["MonitoredCall"]["ExpectedDepartureTime"]
                    .as_str().unwrap_or("");
                let wait_str = parse_wait(exp);
                result.push(format!("{line} → {dest} : {wait_str}"));
            }
        }
    }
    result
}

fn parse_wait(iso: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // iso8601: "2026-03-26T14:30:00Z"
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // Very light ISO parse: find T, extract HH:MM:SS
    if let Some(t_pos) = iso.find('T') {
        let time_part = &iso[t_pos + 1..];
        let parts: Vec<&str> = time_part.splitn(3, ':').collect();
        if parts.len() >= 2 {
            let h: u64 = parts[0].parse().unwrap_or(0);
            let m: u64 = parts[1].parse().unwrap_or(0);
            let s: u64 = parts.get(2).and_then(|p| p.trim_end_matches('Z').parse().ok()).unwrap_or(0);
            let dep_secs = h * 3600 + m * 60 + s;
            let now_secs = now % 86400; // seconds since midnight
            if dep_secs >= now_secs {
                let diff = dep_secs - now_secs;
                if diff < 60 { return "< 1 min".to_owned(); }
                return format!("{} min", diff / 60);
            }
        }
    }
    iso.to_owned()
}

// ─────────────────────────────────────────────────────────────────────────────
// Terrain / monument collision death
// ─────────────────────────────────────────────────────────────────────────────

/// Kills the player on ground impact or collision with a hill / monument.
/// Only active while the IDF map is loaded (resource present).
pub fn idf_terrain_death_system(
    terrain: Option<Res<IdfTerrainData>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    paused: Res<TimePaused>,
    mut death_cause: ResMut<DeathCause>,
    mut next_state: ResMut<NextState<GameState>>,
    game_timer: Res<GameTimer>,
) {
    if paused.0 { return; }
    let Some(terrain) = terrain else { return };
    let Ok(cam) = camera_q.get_single() else { return };
    let pos = cam.translation;

    if pos.y < terrain.floor_y {
        info!("Player hit the IDF ground! Score: {:.1}s", game_timer.0);
        *death_cause = DeathCause::Terrain;
        next_state.set(GameState::Dead);
        return;
    }

    for &(center, hr, vr) in &terrain.kill_zones {
        let d = pos - center;
        let norm = (d.x * d.x + d.z * d.z) / (hr * hr) + (d.y * d.y) / (vr * vr);
        if norm < 1.0 {
            info!("Player flew into IDF terrain obstacle! Score: {:.1}s", game_timer.0);
            *death_cause = DeathCause::Terrain;
            next_state.set(GameState::Dead);
            return;
        }
    }
}

fn make_demo_data(station_id: &str) -> Vec<String> {
    // Semi-random fake times based on station id hash to look varied
    let h = station_id.len() % 7;
    let lines_here: Vec<&str> = IDF_STATIONS.iter()
        .find(|s| s.0 == station_id)
        .map(|s| s.3.to_vec())
        .unwrap_or_default();

    let mut result = Vec::new();
    for (i, &l) in lines_here.iter().take(3).enumerate() {
        let nice_line = l.replace("RER_", "RER ").replace("M", "M");
        let mins_a = 1 + (h + i * 3) % 12;
        let mins_b = mins_a + 4 + i % 5;
        result.push(format!("{nice_line} → Dir A : {mins_a} min"));
        result.push(format!("{nice_line} → Dir B : {mins_b} min"));
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Proximity HUD (bottom-left fallback) and UI spawn functions
// ─────────────────────────────────────────────────────────────────────────────

/// Bottom-left text HUD: shows departures for nearest selected station.
pub fn idf_proximity_hud_system(
    mut hud_q: Query<&mut Text, With<IdfProximityHud>>,
    camera_q: Query<&Transform, With<MainCamera>>,
    station_q: Query<(&Transform, &IdfStation)>,
    next_trains: Res<IdfNextTrains>,
    idf_config: Res<IdfConfig>,
    arm: Res<CameraArmOffset>,
) {
    let Ok(cam_t) = camera_q.get_single() else { return };
    let player_pos = cam_t.translation - arm.0;

    let nearest = station_q.iter()
        .filter(|(_, s)| idf_config.selected_stations.contains(&s.station_idx))
        .min_by(|(a, _), (b, _)| {
            let da = (a.translation - player_pos).length();
            let db = (b.translation - player_pos).length();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

    let Ok(mut hud_text) = hud_q.get_single_mut() else { return };

    if let Some((st, sdata)) = nearest {
        let dist = (st.translation - player_pos).length();
        if dist < PROXIMITY_RADIUS {
            let station = IDF_STATIONS.get(sdata.station_idx);
            let station_name = station.map(|s| s.1).unwrap_or("?");
            let prim_id = station.map(|s| s.2).unwrap_or("");
            let deps = next_trains.departures.get(prim_id)
                .cloned()
                .unwrap_or_else(|| make_demo_data(
                    IDF_STATIONS.get(sdata.station_idx).map(|s| s.0).unwrap_or("")
                ));
            let dist_m = (dist * 0.33) as u32;
            let mut text = format!("\u{2b6f} {station_name}  ({dist_m} m)\n");
            for d in deps.iter().take(6) {
                text.push_str(&format!("  {d}\n"));
            }
            hud_text.sections[0].value = text;
        } else {
            hud_text.sections[0].value.clear();
        }
    } else {
        hud_text.sections[0].value.clear();
    }
}

/// Spawns the IDF proximity HUD (bottom-left text node).
/// Only spawns when the active scene is IdfTransport.
pub fn spawn_idf_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    active_scene: Res<ActiveScene>,
) {
    if active_scene.0 != SceneKind::IdfTransport { return; }
    let font = asset_server.load(crate::setup::resolve_ui_font_path());
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(80.0),
                max_width: Val::Px(300.0),
                ..default()
            },
            text: Text::from_section("", TextStyle {
                font,
                font_size: 15.0,
                color: Color::rgb(0.25, 0.95, 0.85),
            }),
            background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.6)),
            ..default()
        },
        IdfProximityHud,
        SceneEntity,
    ));
}

/// Spawns the floating departure popup (screen-projected above stations).
pub fn spawn_idf_proximity_popup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    active_scene: Res<ActiveScene>,
) {
    if active_scene.0 != SceneKind::IdfTransport { return; }
    let font = asset_server.load(crate::setup::resolve_ui_font_path());
    commands.spawn((
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top:  Val::Px(0.0),
                max_width: Val::Px(340.0),
                ..default()
            },
            text: Text::from_section("", TextStyle {
                font,
                font_size: 14.0,
                color: Color::rgb(1.0, 0.95, 0.80),
            }),
            visibility: Visibility::Hidden,
            background_color: BackgroundColor(Color::rgba(0.0, 0.05, 0.12, 0.80)),
            ..default()
        },
        IdfProximityPopup,
        SceneEntity,
    ));
}
