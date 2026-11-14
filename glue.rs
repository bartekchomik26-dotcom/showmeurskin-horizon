// Show Me Your Skin! for Horizon: make each player's armour see-through or
// hide it, with or without its glint and trims; hide the elytra, shield and
// hat, and name tags. Injected as `crate::game::showmeyourskin`; the hook
// helpers next to this file call the `pub(crate)` functions below. A port of
// enjarai's Fabric mod (MIT).

use super::Game;
use horizon_client::hud::{HudBatch, rgba};
use horizon_client::mobs::{Mob, Part};
use horizon_client::mod_api::{NativeMod, NativeScreenView};
use horizon_client::text::Text;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const ID: &str = "showmeyourskin";
const NAME: &str = "Show Me Your Skin!";
const VERSION: &str = "2.0.4+horizon.1";
const DESCRIPTION: &str = "Make armor see-through or hide it, piece by piece, with or without its glint and trim; hide the elytra, shield, hat and name tags; for everyone or per player. Press O for the Armor Config.";

const KEY_OPEN: (&str, &str, &str) = ("open_settings", "Open Armor Config", "key.keyboard.o");
const KEY_TOGGLE: (&str, &str, &str) = ("global_toggle", "Toggle Mod", "key.keyboard.u");
const KEYS: [(&str, &str, &str); 2] = [KEY_OPEN, KEY_TOGGLE];

// ---------------------------------------------------------------- settings

/// A worn armour piece: how much of it shows (0 hidden, 1 as normal), its
/// enchantment glint, its trim.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ArmorPiece {
    pub opacity: f32,
    pub glint: bool,
    pub trim: bool,
}
impl Default for ArmorPiece {
    fn default() -> Self {
        Self { opacity: 1.0, glint: true, trim: true }
    }
}

/// The elytra, shield and hat: shown or not, and their glint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Piece {
    pub base: bool,
    pub glint: bool,
}
impl Default for Piece {
    fn default() -> Self {
        Self { base: true, glint: true }
    }
}

/// What shows on one player (`ArmorConfig`). The default is vanilla: everything.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ArmorConfig {
    pub head: ArmorPiece,
    pub chest: ArmorPiece,
    pub legs: ArmorPiece,
    pub feet: ArmorPiece,
    pub elytra: Piece,
    pub shield: Piece,
    pub hat: Piece,
    /// The armour shows again while the player is in combat.
    pub show_in_combat: bool,
    pub show_name_tag: bool,
    /// The elytra shows while flying even when hidden.
    pub force_elytra_when_flying: bool,
}
impl Default for ArmorConfig {
    fn default() -> Self {
        Self {
            head: ArmorPiece::default(),
            chest: ArmorPiece::default(),
            legs: ArmorPiece::default(),
            feet: ArmorPiece::default(),
            elytra: Piece::default(),
            shield: Piece::default(),
            hat: Piece::default(),
            show_in_combat: true,
            show_name_tag: true,
            force_elytra_when_flying: true,
        }
    }
}
impl ArmorConfig {
    fn armor(&self, slot: usize) -> &ArmorPiece {
        [&self.head, &self.chest, &self.legs, &self.feet][slot.min(3)]
    }
    fn armor_mut(&mut self, slot: usize) -> &mut ArmorPiece {
        match slot {
            0 => &mut self.head,
            1 => &mut self.chest,
            2 => &mut self.legs,
            _ => &mut self.feet,
        }
    }
    fn piece_mut(&mut self, i: usize) -> &mut Piece {
        match i {
            0 => &mut self.elytra,
            1 => &mut self.shield,
            _ => &mut self.hat,
        }
    }
    /// Nothing changed from vanilla (the armour layers can be left alone).
    fn armor_vanilla(&self) -> bool {
        (0..4).all(|i| *self.armor(i) == ArmorPiece::default())
    }
    /// `CombatLogger.getFade`: `f` of the way back to vanilla (1 in combat).
    fn faded(mut self, f: f32) -> Self {
        if f <= 0.0 {
            return self;
        }
        for i in 0..4 {
            let a = self.armor_mut(i);
            a.opacity += (1.0 - a.opacity) * f;
            if f >= 1.0 {
                *a = ArmorPiece::default();
            }
        }
        if f >= 1.0 {
            self.elytra = Piece::default();
            self.shield = Piece::default();
            self.hat = Piece::default();
        }
        self
    }
}

/// A player's own settings, kept by name (`ModConfig.LocalOverride`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Override {
    pub title: String,
    pub config: ArmorConfig,
}

/// Everything, as `config.json` keeps it (`ModConfig`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ModConfig {
    pub global_enabled: bool,
    /// Seconds a player stays "in combat" after being hurt.
    pub combat_cooldown: f32,
    /// Seconds the armour then takes to fade back to its settings.
    pub fade_out_time: f32,
    pub global: ArmorConfig,
    /// Per player, by lower-case name.
    pub overrides: BTreeMap<String, Override>,
}
impl Default for ModConfig {
    fn default() -> Self {
        Self { global_enabled: true, combat_cooldown: 16.0, fade_out_time: 2.0, global: ArmorConfig::default(), overrides: BTreeMap::new() }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Slider(usize),
    Cooldown,
    Fade,
}

struct State {
    cfg: ModConfig,
    loaded: bool,
    dir: Option<std::path::PathBuf>,
    /// When each player (lower-case name) was last hurt.
    combat: HashMap<String, Instant>,
    /// The menu: who is being edited (None: everyone), the settings page,
    /// the player strip's scroll, a slider held.
    selected: Option<String>,
    settings: bool,
    scroll: f32,
    drag: Option<Drag>,
    open_tries: u32,
    /// The back button was pressed: the screen closes on the next frame.
    close: bool,
    /// The GUI size the screen was last drawn at.
    size: [f32; 2],
    chat: Vec<String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut slot = STATE.lock().unwrap_or_else(|p| p.into_inner());
    let s = slot.get_or_insert_with(|| State {
        cfg: ModConfig::default(),
        loaded: false,
        dir: None,
        combat: HashMap::new(),
        selected: None,
        settings: false,
        scroll: 0.0,
        drag: None,
        open_tries: 0,
        close: false,
        size: [427.0, 240.0],
        chat: Vec::new(),
    });
    f(s)
}

/// Loads `config.json` once, from the mod's data folder.
fn ensure_loaded(game: &Game<'_>) {
    if with(|s| s.loaded) {
        return;
    }
    let dir = game.native_data_dir(ID);
    let cfg = std::fs::read_to_string(dir.join("config.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<ModConfig>(&t).ok())
        .unwrap_or_default();
    with(|s| {
        s.cfg = sanitized(cfg);
        s.dir = Some(dir);
        s.loaded = true;
    });
}

fn sanitized(mut c: ModConfig) -> ModConfig {
    let clamp = |v: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(0.0, hi) } else { d };
    c.combat_cooldown = clamp(c.combat_cooldown, 120.0, 16.0);
    c.fade_out_time = clamp(c.fade_out_time, 30.0, 2.0);
    for cfg in std::iter::once(&mut c.global).chain(c.overrides.values_mut().map(|o| &mut o.config)) {
        for i in 0..4 {
            let a = cfg.armor_mut(i);
            a.opacity = clamp(a.opacity, 1.0, 1.0);
        }
    }
    c
}

fn save(s: &State) {
    if let Some(dir) = &s.dir {
        let _ = std::fs::create_dir_all(dir);
        if let Ok(text) = serde_json::to_string_pretty(&s.cfg) {
            let tmp = dir.join("config.json.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, dir.join("config.json"));
            }
        }
    }
}

// ---------------------------------------------------------------- who is who

thread_local! {
    /// While the menu draws its preview: the config being edited, shown on the local player.
    static PREVIEW: RefCell<Option<ArmorConfig>> = const { RefCell::new(None) };
}

/// The player a model is: `Some(name)` in lower case, the local player's own
/// for its model; None for anything that isn't a player.
fn who(game: &Game<'_>, mob: Option<&Mob>) -> Option<String> {
    let own = || Some(game.own_profile_name().to_lowercase());
    let Some(m) = mob else { return own() };
    if game.player_model.as_ref().is_some_and(|p| p.id == m.id && p.kind.id == m.kind.id) {
        return own();
    }
    if &*m.kind.id != "minecraft:player" {
        return None;
    }
    Some(game.remote_player_name(m.id).map_or_else(|| format!("player#{}", m.id), str::to_lowercase))
}

/// `ModConfig.getApplicable`: off, the player's own settings or everyone's,
/// back to vanilla in combat (and fading out of it) when set so.
fn applicable(name: &str) -> ArmorConfig {
    with(|s| {
        if !s.cfg.global_enabled {
            return ArmorConfig::default();
        }
        let cfg = s.cfg.overrides.get(name).map_or_else(|| s.cfg.global.clone(), |o| o.config.clone());
        if !cfg.show_in_combat {
            return cfg;
        }
        let Some(hurt) = s.combat.get(name) else { return cfg };
        let since = hurt.elapsed().as_secs_f32();
        let fade = s.cfg.fade_out_time.max(0.001);
        let f = if since < s.cfg.combat_cooldown { 1.0 } else { 1.0 - (since - s.cfg.combat_cooldown) / fade };
        cfg.faded(f.clamp(0.0, 1.0))
    })
}

fn config_for(game: &Game<'_>, mob: Option<&Mob>) -> Option<ArmorConfig> {
    ensure_loaded(game);
    let name = who(game, mob)?;
    let own = match mob {
        None => true,
        Some(m) => game.player_model.as_ref().is_some_and(|p| p.id == m.id),
    };
    if own && let Some(p) = PREVIEW.with(|p| p.borrow().clone()) {
        return Some(p);
    }
    Some(applicable(&name))
}

/// The player holding an item, from the item's owner (its position and type).
fn owner_player<'g>(game: &'g Game<'_>, owner: &horizon_assets::item_model::OwnerState) -> Option<&'g Mob> {
    if owner.entity_type != "minecraft:player" {
        return None;
    }
    let at = |m: &Mob| [m.position.x, m.position.y, m.position.z] == owner.position;
    game.player_model
        .as_ref()
        .filter(|m| at(m))
        .or_else(|| game.mobs.iter().find(|m| &*m.kind.id == "minecraft:player" && at(m)))
}

// ---------------------------------------------------------------- armour

/// How one player's armour layers come out (taken by [`apply_armor`]).
pub(crate) struct ArmorPlan {
    /// The player armour models, helmet to boots (by address).
    parts: [usize; 4],
    /// The atlas rects of the trims drawn this frame.
    trims: Vec<[f32; 4]>,
    cfg: ArmorConfig,
}

thread_local! {
    static PLAN: RefCell<Option<ArmorPlan>> = const { RefCell::new(None) };
}

/// Before a player's armour is laid out: what its config changes.
pub(crate) fn armor_start(game: &Game<'_>, mob: Option<&Mob>) {
    let plan = config_for(game, mob).filter(|c| !c.armor_vanilla()).map(|cfg| {
        let part = |slot: &str| game.armor_stand_assets.player.get(slot).map_or(0, |p| p as *const Part as usize);
        let sprites = &game.gui.atlas.sprites;
        ArmorPlan {
            parts: [part("helmet"), part("chestplate"), part("leggings"), part("boots")],
            // Trims by sprite name: TrimSlots has no iter() on Nightly 2026-11-13.
            trims: sprites.iter().filter(|(n, _)| n.contains("trims/")).map(|(_, r)| [r.u0, r.v0, r.u1, r.v1]).collect(),
            cfg,
        }
    });
    PLAN.with(|p| *p.borrow_mut() = plan);
}

/// The armour layers (model, texture rect, colour, overlay, emissive) as the
/// player's config has them: see-through (the colour's alpha), without the
/// glints and trims it turns off, and without the pieces at 0%.
pub(crate) fn apply_armor<'a>(
    mut layers: Vec<(&'a Part, [f32; 4], u32, u32, bool)>,
) -> Vec<(&'a Part, [f32; 4], u32, u32, bool)> {
    let Some(plan) = PLAN.with(|p| p.borrow_mut().take()) else { return layers };
    let glint = horizon_client::mobs::render::ARMOR_GLINT;
    layers.retain_mut(|(part, uv, color, overlay, _)| {
        let addr = *part as *const Part as usize;
        let Some(slot) = plan.parts.iter().position(|&p| p != 0 && p == addr) else { return true };
        let piece = plan.cfg.armor(slot);
        let trim = plan.trims.iter().any(|t| t == uv);
        if piece.opacity < 0.004 || (trim && !piece.trim) {
            return false;
        }
        if !piece.glint && *overlay == glint {
            *overlay = 0;
        }
        if piece.opacity < 0.999 {
            // The colour is ABGR: its alpha is the top byte.
            let a = ((*color >> 24) as f32 * piece.opacity).round() as u32;
            *color = (*color & 0x00FF_FFFF) | (a.min(255) << 24);
        }
        true
    });
    layers
}

// ---------------------------------------------------------------- elytra, hat, shield, name

/// `WingsLayer` on a player: hidden (unless flying, when set so), or drawn here without its glint.
#[allow(clippy::too_many_arguments)]
pub(crate) fn elytra(
    game: &Game<'_>,
    mob: &Mob,
    chest: Option<&str>,
    camera: glam::DVec3,
    partial: f32,
    light: u32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) -> std::ops::ControlFlow<()> {
    use std::ops::ControlFlow::{Break, Continue};
    let Some(cfg) = config_for(game, Some(mob)) else { return Continue(()) };
    let own = game.player_model.as_ref().is_some_and(|p| p.id == mob.id);
    let flying = mob.extra.fall_flying || (own && game.player.as_ref().is_some_and(|p| p.fall_flying));
    if !(cfg.elytra.base || (cfg.force_elytra_when_flying && flying)) {
        return Break(());
    }
    if cfg.elytra.glint {
        return Continue(());
    }
    // As `player_elytra_geometry` draws it, with no glint overlay.
    let stack = mob.extra.armor[1].as_ref().filter(|s| chest == Some(&*s.item));
    game.wings_geometry(
        chest,
        stack,
        mob.cape.as_deref(),
        false,
        horizon_client::mobs::render::base_transform(mob, partial, camera),
        mob.extra.elytra.angles(partial),
        mob.extra.crouching,
        light,
        0,
        out,
    );
    Break(())
}

/// `CustomHeadLayer` on a player: nothing when the hat is hidden.
pub(crate) fn hat(game: &Game<'_>, mob: &Mob) -> std::ops::ControlFlow<()> {
    match config_for(game, Some(mob)) {
        Some(cfg) if !cfg.hat.base => std::ops::ControlFlow::Break(()),
        _ => std::ops::ControlFlow::Continue(()),
    }
}

/// An item resolved for a player's hand or head: a hidden shield draws
/// nothing; a shield or hat without glint loses its foil.
pub(crate) fn shield(
    game: &Game<'_>,
    stack: &horizon_client::items::ItemStack,
    display: horizon_assets::item_model::DisplayContext,
    owner: Option<&horizon_assets::item_model::OwnerState>,
    mut state: horizon_client::item_render::ItemRenderState,
) -> horizon_client::item_render::ItemRenderState {
    use horizon_assets::item_model::DisplayContext as D;
    let (is_shield, head) = (&*stack.item == "minecraft:shield", display == D::Head);
    if !is_shield && !head {
        return state;
    }
    let cfg = match display {
        D::FirstPersonLeftHand | D::FirstPersonRightHand => config_for(game, None),
        D::ThirdPersonLeftHand | D::ThirdPersonRightHand | D::Head => {
            owner.and_then(|o| owner_player(game, o)).and_then(|m| config_for(game, Some(m)))
        }
        _ => None,
    };
    let Some(cfg) = cfg else { return state };
    let piece = if head { cfg.hat } else { cfg.shield };
    if !piece.base && !head {
        state.layers.clear();
    } else if !piece.glint {
        for layer in &mut state.layers {
            layer.foil = horizon_assets::item_model::FoilType::None;
        }
        state.cit_glint = None;
    }
    state
}

/// A player's name tag, hidden when its config says so.
pub(crate) fn name_tag(game: &Game<'_>, entity_id: u32, kind: &str, visible: &mut bool) {
    if kind != "minecraft:player" || !*visible {
        return;
    }
    let mob = game
        .player_model
        .as_ref()
        .filter(|m| m.id == entity_id)
        .or_else(|| game.mobs.iter().find(|m| m.id == entity_id && &*m.kind.id == "minecraft:player"));
    if let Some(m) = mob
        && let Some(cfg) = config_for(game, Some(m))
        && !cfg.show_name_tag
    {
        *visible = false;
    }
}

/// Whether the carved pumpkin's blur shows (no: the local player's hat is hidden).
#[allow(dead_code)]
pub(crate) fn pumpkin_overlay(game: &Game<'_>) -> bool {
    config_for(game, None).is_none_or(|c| c.hat.base)
}

// ---------------------------------------------------------------- hooks: list, keys, frame

pub(crate) fn listed(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: ID.into(),
        title: format!("{NAME} {VERSION}"),
        description: DESCRIPTION.into(),
        keys: KEYS.iter().map(|(n, l, d)| [(*n).to_owned(), (*l).to_owned(), (*d).to_owned()]).collect(),
    });
}

pub(crate) fn settings_page(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != ID {
        return;
    }
    ensure_loaded(game);
    let on = with(|s| s.cfg.global_enabled);
    let key = game.native_mod_key(ID, KEY_OPEN.0, KEY_OPEN.2);
    buttons.push(format!("Armor customisation: {}", if on { "ON" } else { "OFF" }));
    buttons.push(format!("Open Armor Config ({})", key_label(&key)));
}

pub(crate) fn settings_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != ID {
        return;
    }
    ensure_loaded(game);
    match button {
        0 => toggle(game),
        1 => open(game),
        _ => {}
    }
}

fn key_label(key: &str) -> String {
    key.rsplit('.').next().unwrap_or(key).to_uppercase()
}

fn bound(g: &Game<'_>, k: (&str, &str, &str), key: &str) -> bool {
    g.native_mod_key(ID, k.0, k.2) == key
}

pub(crate) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_window() || !game.native_on(ID) {
        return;
    }
    let mine = game.native_mod_screen() == Some(ID);
    if bound(game, KEY_OPEN, key) {
        if mine {
            game.native_mod_screen_close(ID);
        } else if game.native_playing() {
            ensure_loaded(game);
            open(game);
        }
    } else if bound(game, KEY_TOGGLE, key) && game.native_playing() {
        ensure_loaded(game);
        toggle(game);
    }
}

fn toggle(game: &mut Game<'_>) {
    let on = with(|s| {
        s.cfg.global_enabled = !s.cfg.global_enabled;
        save(s);
        s.cfg.global_enabled
    });
    game.native_chat(if on {
        "[Show Me Your Skin] Enabled armor rendering customisation."
    } else {
        "[Show Me Your Skin] Disabled armor rendering customisation."
    });
}

fn open(game: &mut Game<'_>) {
    with(|s| {
        s.drag = None;
        s.settings = false;
    });
    if !game.native_mod_screen_open(ID) {
        with(|s| s.open_tries = 20);
    }
}

/// Once a frame: a menu waiting to open or close, chat lines, and who was just
/// hurt (`CombatLogger`).
pub(crate) fn frame(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    if with(|s| std::mem::take(&mut s.close)) {
        game.native_mod_screen_close(ID);
    }
    let tries = with(|s| std::mem::take(&mut s.open_tries));
    if tries > 0 && !game.native_mod_screen_open(ID) {
        with(|s| s.open_tries = tries - 1);
    }
    for line in with(|s| std::mem::take(&mut s.chat)) {
        game.native_chat(&line);
    }
    if !game.native_on(ID) {
        return;
    }
    let g: &Game<'_> = game;
    let mut hurt: Vec<String> = Vec::new();
    if let Some(m) = g.player_model.as_ref()
        && m.hurt_time > 0
    {
        hurt.push(g.own_profile_name().to_lowercase());
    }
    for m in g.mobs.iter().filter(|m| m.hurt_time > 0 && &*m.kind.id == "minecraft:player") {
        if let Some(n) = who(g, Some(m)) {
            hurt.push(n);
        }
    }
    let now = Instant::now();
    with(|s| {
        for n in hurt {
            s.combat.insert(n, now);
        }
        let keep = Duration::from_secs_f32(s.cfg.combat_cooldown + s.cfg.fade_out_time + 1.0);
        s.combat.retain(|_, t| t.elapsed() < keep);
    });
}

// ---------------------------------------------------------------- the armor config screen

fn argb(rgb: u32, alpha: f32) -> u32 {
    ((alpha.clamp(0.0, 1.0) * 255.0).round() as u32) << 24 | (rgb & 0xFF_FFFF)
}
fn packed(c: u32) -> u32 {
    rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, (c >> 24) as u8)
}
fn fill(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4], color: u32) {
    if r[2] > 0.0 && r[3] > 0.0 && (color >> 24) > 0 {
        g.native_fill(b, r, color);
    }
}
fn text_w(g: &Game<'_>, s: &str) -> f32 {
    g.gui.atlas.font.width_text(&Text::literal(s.to_owned()))
}
fn label(g: &Game<'_>, b: &mut HudBatch, s: &str, x: f32, y: f32, color: u32, shadow: bool) {
    b.text_styled(&g.gui.atlas, &Text::literal(s.to_owned()), x, y, packed(color), shadow);
}
fn inside(p: [f32; 2], r: [f32; 4]) -> bool {
    p[0] >= r[0] && p[0] < r[0] + r[2] && p[1] >= r[1] && p[1] < r[1] + r[3]
}

/// A raised panel in the game's own grey, as the inventory draws it.
fn bevel(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4]) {
    let [x, y, w, h] = r;
    fill(g, b, [x + 1.0, y + 1.0, w - 2.0, h - 2.0], argb(0xC6C6C6, 1.0));
    fill(g, b, [x + 1.0, y + 1.0, w - 3.0, 2.0], argb(0xFFFFFF, 1.0));
    fill(g, b, [x + 1.0, y + 1.0, 2.0, h - 3.0], argb(0xFFFFFF, 1.0));
    fill(g, b, [x + 3.0, y + h - 3.0, w - 4.0, 2.0], argb(0x555555, 1.0));
    fill(g, b, [x + w - 3.0, y + 3.0, 2.0, h - 4.0], argb(0x555555, 1.0));
    fill(g, b, [x + 1.0, y, w - 2.0, 1.0], argb(0x000000, 1.0));
    fill(g, b, [x + 1.0, y + h - 1.0, w - 2.0, 1.0], argb(0x000000, 1.0));
    fill(g, b, [x, y + 1.0, 1.0, h - 2.0], argb(0x000000, 1.0));
    fill(g, b, [x + w - 1.0, y + 1.0, 1.0, h - 2.0], argb(0x000000, 1.0));
}
/// A sunken area inside a panel.
fn inset(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4], color: u32) {
    let [x, y, w, h] = r;
    fill(g, b, [x, y, w, h], argb(0x373737, 1.0));
    fill(g, b, [x + 1.0, y + 1.0, w - 1.0, h - 1.0], argb(0xFFFFFF, 1.0));
    fill(g, b, [x + 1.0, y + 1.0, w - 2.0, h - 2.0], argb(color, 1.0));
}
fn button(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4], hover: bool, enabled: bool) {
    let sprite = match (enabled, hover) {
        (false, _) => "gui/sprites/widget/button_disabled",
        (true, true) => "gui/sprites/widget/button_highlighted",
        (true, false) => "gui/sprites/widget/button",
    };
    if g.gui.atlas.sprites.contains_key(sprite) {
        horizon_client::ui::nine_slice(b, &g.gui.atlas, sprite, r[0], r[1], r[2], r[3], u32::MAX);
    } else {
        fill(g, b, r, argb(if hover { 0x8A8AA0 } else { 0x6E6E6E }, 1.0));
    }
}
fn item(g: &Game<'_>, b: &mut HudBatch, id: &str, x: f32, y: f32, size: f32) {
    let _ = g.native_item(b, id, [x, y], size);
}

/// The panels' width.
const PANEL_W: f32 = 276.0;
const PIECE_NAMES: [&str; 4] = ["Helmet", "Chestplate", "Leggings", "Boots"];
/// The toggles under the sliders: (item icon, tooltip).
const TOGGLES: [(&str, &str); 6] = [
    ("minecraft:iron_sword", "Show armor in combat"),
    ("minecraft:name_tag", "Show player name tag"),
    ("minecraft:shield", "Show shield\nRight-click: its glint"),
    ("minecraft:elytra", "Show elytra\nRight-click: its glint"),
    ("minecraft:carved_pumpkin", "Show hat (heads, pumpkins)\nRight-click: its glint"),
    ("minecraft:firework_rocket", "Show elytra while flying"),
];

/// What a click does.
#[derive(Clone, Debug, PartialEq)]
enum Act {
    Select(Option<String>),
    Slider(usize),
    Glint(usize),
    Trim(usize),
    Toggle(usize),
    Back,
    Settings,
    GlobalToggle,
    Cooldown,
    Fade,
    CreateOverride,
    DeleteOverride,
}

/// The screen's parts, placed for a GUI of `w` by `h`.
struct Layout {
    top: [f32; 4],
    strip: [f32; 4],
    main: [f32; 4],
    sliders_box: [f32; 4],
    toggles_box: [f32; 4],
    preview: [f32; 4],
    back: [f32; 4],
    settings: [f32; 4],
    /// (rect, action, tooltip)
    hits: Vec<([f32; 4], Act, String)>,
    /// The player strip: (rect, key, name).
    players: Vec<([f32; 4], Option<String>, String)>,
}

fn players(g: &Game<'_>, cfg: &ModConfig) -> Vec<(Option<String>, String)> {
    let own = g.own_profile_name();
    let mut out: Vec<(Option<String>, String)> = vec![(None, "Global".into()), (Some(own.to_lowercase()), own)];
    for (_, name) in g.remote_player_names() {
        let k = name.to_lowercase();
        if !out.iter().any(|(p, _)| p.as_deref() == Some(&k)) {
            out.push((Some(k), name));
        }
    }
    for (k, o) in &cfg.overrides {
        if !out.iter().any(|(p, _)| p.as_deref() == Some(k)) {
            out.push((Some(k.clone()), o.title.clone()));
        }
    }
    out
}

fn layout(g: &Game<'_>, w: f32, h: f32) -> Layout {
    let (cfg, selected, settings, scroll) = with(|s| (s.cfg.clone(), s.selected.clone(), s.settings, s.scroll));
    let (pw, top_h, main_h) = (PANEL_W, 40.0, 172.0);
    let total_h = top_h + 4.0 + main_h;
    let x = ((w - pw) / 2.0 + 12.0).round();
    let y = ((h - total_h) / 2.0).round().max(2.0);
    let top = [x, y, pw, top_h];
    let strip = [x + 6.0, y + 14.0, pw - 12.0, 22.0];
    let main = [x, y + top_h + 4.0, pw, main_h];
    let back = [x - 26.0, y, 22.0, 22.0];
    let settings_btn = [x - 26.0, y + 26.0, 22.0, 22.0];
    let left_w = 150.0;
    let sliders_box = [main[0] + 6.0, main[1] + 6.0, left_w, 4.0 * 22.0 + 4.0];
    let toggles_box = [main[0] + 6.0, sliders_box[1] + sliders_box[3] + 4.0, left_w, main_h - sliders_box[3] - 16.0];
    let preview = [main[0] + 6.0 + left_w + 4.0, main[1] + 6.0, pw - left_w - 16.0, main_h - 12.0];
    let mut hits = Vec::new();
    hits.push((back, Act::Back, "Back".to_owned()));
    hits.push((settings_btn, Act::Settings, if settings { "Armor" } else { "Settings" }.to_owned()));
    // The player strip: Global, you, everyone here and everyone with settings.
    let mut ps = Vec::new();
    let mut px = strip[0] + 2.0 - scroll;
    for (key, name) in players(g, &cfg) {
        let r = [px, strip[1] + 2.0, 18.0, 18.0];
        if r[0] >= strip[0] && r[0] + r[2] <= strip[0] + strip[2] {
            let overridden = key.as_ref().is_some_and(|k| cfg.overrides.contains_key(k));
            let tip = if key.is_none() { "Global".to_owned() } else if overridden { format!("{name}\nOverridden") } else { name.clone() };
            hits.push((r, Act::Select(key.clone()), tip));
        }
        ps.push((r, key, name));
        px += 20.0;
    }
    if settings {
        let bx = main[0] + 10.0;
        let mut by = main[1] + 24.0;
        hits.push(([bx, by, pw - 20.0, 20.0], Act::GlobalToggle, "Global toggle (U in game)".into()));
        by += 34.0;
        hits.push(([bx, by, pw - 20.0, 20.0], Act::Cooldown, "How long armor shows after being hurt".into()));
        by += 34.0;
        hits.push(([bx, by, pw - 20.0, 20.0], Act::Fade, "How long it then takes to fade out".into()));
    } else {
        let editing = match &selected {
            None => true,
            Some(k) => cfg.overrides.contains_key(k),
        };
        if editing {
            for i in 0..4 {
                let ry = sliders_box[1] + 3.0 + i as f32 * 22.0;
                hits.push(([sliders_box[0] + 3.0, ry, 100.0, 20.0], Act::Slider(i), String::new()));
                hits.push(([sliders_box[0] + 105.0, ry, 20.0, 20.0], Act::Glint(i), "Enable enchantment glint".into()));
                hits.push(([sliders_box[0] + 127.0, ry, 20.0, 20.0], Act::Trim(i), "Show armor trim".into()));
            }
            for (i, (_, tip)) in TOGGLES.iter().enumerate() {
                let (col, row) = (i % 4, i / 4);
                let r = [toggles_box[0] + 5.0 + col as f32 * 36.0, toggles_box[1] + 5.0 + row as f32 * 26.0, 22.0, 22.0];
                hits.push((r, Act::Toggle(i), (*tip).to_owned()));
            }
            if selected.is_some() {
                hits.push(([preview[0] + preview[2] - 16.0, preview[1] + 3.0, 13.0, 13.0], Act::DeleteOverride, "Delete override".into()));
            }
        } else {
            hits.push((
                [sliders_box[0] + 3.0, sliders_box[1] + 30.0, left_w - 6.0, 20.0],
                Act::CreateOverride,
                "Open armor override\n(not visible to other players)".into(),
            ));
        }
    }
    Layout { top, strip, main, sliders_box, toggles_box, preview, back, settings: settings_btn, hits, players: ps }
}

fn mine(game: &Game<'_>) -> bool {
    game.native_mod_screen() == Some(ID)
}

/// The config the screen edits: everyone's, or the selected player's override.
fn editing_cfg(s: &State) -> Option<ArmorConfig> {
    match &s.selected {
        None => Some(s.cfg.global.clone()),
        Some(k) => s.cfg.overrides.get(k).map(|o| o.config.clone()),
    }
}
fn editing(s: &mut State) -> Option<&mut ArmorConfig> {
    match s.selected.clone() {
        None => Some(&mut s.cfg.global),
        Some(k) => s.cfg.overrides.get_mut(&k).map(|o| &mut o.config),
    }
}

pub(crate) fn screen_draw(game: &Game<'_>, b: &mut HudBatch, screen: &NativeScreenView<'_>) {
    if !mine(game) {
        return;
    }
    ensure_loaded(game);
    let m = screen.mouse;
    with(|s| s.size = [b.width, b.height]);
    let lay = layout(game, b.width, b.height);
    let (cfg, selected, settings, edited) = with(|s| (s.cfg.clone(), s.selected.clone(), s.settings, editing_cfg(s)));
    fill(game, b, [0.0, 0.0, b.width, b.height], argb(0x000000, 0.45));
    // The side buttons: back, and the settings (a comparator; a chestplate back to the armour).
    for (r, icon) in [(lay.back, None), (lay.settings, Some(if settings { "minecraft:iron_chestplate" } else { "minecraft:comparator" }))] {
        button(game, b, r, inside(m, r), true);
        match icon {
            Some(id) => item(game, b, id, r[0] + 3.0, r[1] + 3.0, 16.0),
            None => label(game, b, "<", r[0] + 8.0, r[1] + 7.0, argb(0xFFFFFF, 1.0), true),
        }
    }
    // The top: the title and the player strip.
    bevel(game, b, lay.top);
    label(game, b, "Armor Config", lay.top[0] + 7.0, lay.top[1] + 4.0, argb(0x404040, 1.0), false);
    inset(game, b, lay.strip, 0x1E1E1E);
    for (r, key, _) in &lay.players {
        if r[0] < lay.strip[0] || r[0] + r[2] > lay.strip[0] + lay.strip[2] {
            continue;
        }
        let has = key.as_ref().is_some_and(|k| cfg.overrides.contains_key(k));
        if inside(m, *r) {
            fill(game, b, *r, argb(0xFFFFFF, 0.15));
        }
        let icon = match key {
            None => "minecraft:compass",
            Some(_) if has => "minecraft:player_head",
            Some(_) => "minecraft:skeleton_skull",
        };
        item(game, b, icon, r[0] + 1.0, r[1] + 1.0, 16.0);
        if *key == selected {
            corners(game, b, *r);
        }
    }
    // The main panel.
    bevel(game, b, lay.main);
    if settings {
        let bx = lay.main[0] + 10.0;
        label(game, b, "Settings", bx, lay.main[1] + 8.0, argb(0x404040, 1.0), false);
        for (r, act, _) in &lay.hits {
            let (text, v) = match act {
                Act::GlobalToggle => (format!("Armor customisation: {}", if cfg.global_enabled { "ON" } else { "OFF" }), None),
                Act::Cooldown => (format!("Combat cooldown: {:.0} s", cfg.combat_cooldown), Some(cfg.combat_cooldown / 60.0)),
                Act::Fade => (format!("Fade out time: {:.1} s", cfg.fade_out_time), Some(cfg.fade_out_time / 10.0)),
                _ => continue,
            };
            slider_or_button(game, b, *r, &text, v, inside(m, *r));
        }
    } else {
        inset(game, b, lay.sliders_box, 0x8B8B8B);
        inset(game, b, lay.toggles_box, 0x8B8B8B);
        match &edited {
            Some(c) => {
                for (r, act, _) in &lay.hits {
                    let hover = inside(m, *r);
                    match act {
                        Act::Slider(i) => {
                            let v = c.armor(*i).opacity;
                            slider_or_button(game, b, *r, &format!("{}: {:.0}%", PIECE_NAMES[*i], v * 100.0), Some(v), hover);
                        }
                        Act::Glint(i) => {
                            button(game, b, *r, hover, true);
                            item(game, b, "minecraft:enchanted_book", r[0] + 2.0, r[1] + 2.0, 16.0);
                            if !c.armor(*i).glint {
                                cross(game, b, *r);
                            }
                        }
                        Act::Trim(i) => {
                            button(game, b, *r, hover, true);
                            item(game, b, "minecraft:coast_armor_trim_smithing_template", r[0] + 2.0, r[1] + 2.0, 16.0);
                            if !c.armor(*i).trim {
                                cross(game, b, *r);
                            }
                        }
                        Act::Toggle(i) => {
                            let (on, glint) = match i {
                                0 => (c.show_in_combat, true),
                                1 => (c.show_name_tag, true),
                                2 => (c.shield.base, c.shield.glint),
                                3 => (c.elytra.base, c.elytra.glint),
                                4 => (c.hat.base, c.hat.glint),
                                _ => (c.force_elytra_when_flying, true),
                            };
                            button(game, b, *r, hover, true);
                            item(game, b, TOGGLES[*i].0, r[0] + 3.0, r[1] + 3.0, 16.0);
                            if !on {
                                cross(game, b, *r);
                            } else if !glint {
                                // A grey dot: shown without its glint.
                                fill(game, b, [r[0] + r[2] - 6.0, r[1] + 2.0, 4.0, 4.0], argb(0x7F7F7F, 1.0));
                            }
                        }
                        _ => {}
                    }
                }
            }
            None => {
                label(game, b, "Uses the global config.", lay.sliders_box[0] + 5.0, lay.sliders_box[1] + 10.0, argb(0xFFFFFF, 1.0), true);
                for (r, act, _) in &lay.hits {
                    if *act == Act::CreateOverride {
                        slider_or_button(game, b, *r, "Open override", None, inside(m, *r));
                    }
                }
            }
        }
        // The preview: the local player wearing the config being edited.
        inset(game, b, lay.preview, 0x000000);
        let p = lay.preview;
        let name = match &selected {
            None => "Global".to_owned(),
            Some(k) => lay.players.iter().find(|(_, key, _)| key.as_deref() == Some(k)).map_or(k.clone(), |x| x.2.clone()),
        };
        label(game, b, &name, p[0] + 4.0, p[1] + 4.0, argb(0x5A5A5A, 1.0), false);
        let shown = edited.clone().unwrap_or_else(|| cfg.global.clone());
        PREVIEW.with(|c| *c.borrow_mut() = Some(shown));
        game.draw_player_preview(b, p[0] + 2.0, p[1] + 14.0, p[0] + p[2] - 2.0, p[1] + p[3] - 4.0, 52.0, 0.0625);
        PREVIEW.with(|c| *c.borrow_mut() = None);
        // Over the preview: the override's delete button.
        if let Some(r) = hit_rect(&lay, &Act::DeleteOverride) {
            fill(game, b, r, argb(if inside(m, r) { 0xC03030 } else { 0x802020 }, 1.0));
            label(game, b, "x", r[0] + 4.0, r[1] + 2.0, argb(0xFFFFFF, 1.0), false);
        }
    }
    // A tooltip for what the mouse is over.
    if let Some((_, _, tip)) = lay.hits.iter().find(|(r, _, t)| !t.is_empty() && inside(m, *r)) {
        tooltip(game, b, tip, m);
    }
}

/// The white corner brackets round the player being edited.
fn corners(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4]) {
    let [x, y, w, h] = [r[0] - 1.0, r[1] - 1.0, r[2] + 2.0, r[3] + 2.0];
    let c = argb(0xFFFFFF, 1.0);
    for (cx, right) in [(x, false), (x + w, true)] {
        for (cy, bottom) in [(y, false), (y + h, true)] {
            let hx = if right { cx - 5.0 } else { cx };
            let vy = if bottom { cy - 1.0 } else { cy };
            fill(g, b, [hx, vy, 5.0, 1.0], c);
            let vx = if right { cx - 1.0 } else { cx };
            let vy2 = if bottom { cy - 5.0 } else { cy };
            fill(g, b, [vx, vy2, 1.0, 5.0], c);
        }
    }
}

/// A red bar across a button: off.
fn cross(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4]) {
    for i in 0..15 {
        let t = i as f32;
        fill(g, b, [r[0] + 3.0 + t, r[1] + 3.0 + t, 2.0, 2.0], argb(0xE03030, 0.95));
    }
}

/// A vanilla slider (with `value` in 0..1) or, without one, a button, labelled.
fn slider_or_button(g: &Game<'_>, b: &mut HudBatch, r: [f32; 4], text: &str, value: Option<f32>, hover: bool) {
    match value {
        Some(v) => {
            let bg = "gui/sprites/widget/slider";
            if g.gui.atlas.sprites.contains_key(bg) {
                horizon_client::ui::nine_slice(b, &g.gui.atlas, bg, r[0], r[1], r[2], r[3], u32::MAX);
            } else {
                fill(g, b, r, argb(0x000000, 1.0));
            }
            let hx = r[0] + (r[2] - 8.0) * v.clamp(0.0, 1.0);
            let handle = if hover { "gui/sprites/widget/slider_handle_highlighted" } else { "gui/sprites/widget/slider_handle" };
            if g.gui.atlas.sprites.contains_key(handle) {
                horizon_client::ui::nine_slice(b, &g.gui.atlas, handle, hx, r[1], 8.0, r[3], u32::MAX);
            } else {
                button(g, b, [hx, r[1], 8.0, r[3]], hover, true);
            }
        }
        None => button(g, b, r, hover, true),
    }
    let tw = text_w(g, text);
    let x = r[0] + ((r[2] - tw) / 2.0).max(2.0);
    let start = b.vertices.len();
    label(g, b, text, x, r[1] + (r[3] - 8.0) / 2.0, argb(0xFFFFFF, 1.0), true);
    if tw > r[2] - 4.0 {
        b.clip_since(start, r[0] + 2.0, r[1], r[0] + r[2] - 2.0, r[1] + r[3]);
    }
}

fn tooltip(g: &Game<'_>, b: &mut HudBatch, text: &str, m: [f32; 2]) {
    let lines: Vec<&str> = text.lines().collect();
    let w = lines.iter().map(|l| text_w(g, l)).fold(0.0, f32::max) + 8.0;
    let h = lines.len() as f32 * 10.0 + 6.0;
    let x = (m[0] + 10.0).min(b.width - w - 2.0);
    let y = (m[1] - 14.0).max(2.0);
    fill(g, b, [x, y, w, h], argb(0x100010, 0.94));
    fill(g, b, [x, y, w, 1.0], argb(0x5000FF, 0.6));
    fill(g, b, [x, y + h - 1.0, w, 1.0], argb(0x28007F, 0.6));
    for (i, l) in lines.iter().enumerate() {
        label(g, b, l, x + 4.0, y + 4.0 + i as f32 * 10.0, argb(0xFFFFFF, 1.0), true);
    }
}

/// Sets a slider from the mouse's x.
fn set_slider(s: &mut State, drag: Drag, r: [f32; 4], x: f32) {
    let v = ((x - r[0] - 4.0) / (r[2] - 8.0)).clamp(0.0, 1.0);
    match drag {
        Drag::Slider(i) => {
            if let Some(cfg) = editing(s) {
                // In steps of 5%.
                cfg.armor_mut(i).opacity = (v * 20.0).round() / 20.0;
            }
        }
        Drag::Cooldown => s.cfg.combat_cooldown = (v * 60.0).round(),
        Drag::Fade => s.cfg.fade_out_time = (v * 100.0).round() / 10.0,
    }
}

fn hit_rect(lay: &Layout, want: &Act) -> Option<[f32; 4]> {
    lay.hits.iter().find(|(_, a, _)| a == want).map(|(r, _, _)| *r)
}

pub(crate) fn screen_click(game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32, canceled: &mut bool) {
    if !mine(game) {
        return;
    }
    *canceled = true;
    if button != 0 && button != 1 {
        return;
    }
    ensure_loaded(game);
    let m = screen.mouse;
    let [w, h] = with(|s| s.size);
    let lay = layout(game, w, h);
    let Some((r, act, _)) = lay.hits.iter().find(|(r, _, _)| inside(m, *r)).cloned() else { return };
    let right = button == 1;
    let own = game.own_profile_name();
    let names = players(game, &with(|s| s.cfg.clone()));
    with(|s| {
        match act {
            Act::Select(k) => {
                s.selected = k;
                s.settings = false;
            }
            Act::Back => {
                s.close = true;
                s.drag = None;
            }
            Act::Settings => s.settings = !s.settings,
            Act::GlobalToggle => s.cfg.global_enabled = !s.cfg.global_enabled,
            Act::Cooldown => {
                set_slider(s, Drag::Cooldown, r, m[0]);
                s.drag = Some(Drag::Cooldown);
            }
            Act::Fade => {
                set_slider(s, Drag::Fade, r, m[0]);
                s.drag = Some(Drag::Fade);
            }
            Act::Slider(i) => {
                set_slider(s, Drag::Slider(i), r, m[0]);
                s.drag = Some(Drag::Slider(i));
            }
            Act::Glint(i) => {
                if let Some(c) = editing(s) {
                    let a = c.armor_mut(i);
                    a.glint = !a.glint;
                }
            }
            Act::Trim(i) => {
                if let Some(c) = editing(s) {
                    let a = c.armor_mut(i);
                    a.trim = !a.trim;
                }
            }
            Act::Toggle(i) => {
                if let Some(c) = editing(s) {
                    match i {
                        0 => c.show_in_combat = !c.show_in_combat,
                        1 => c.show_name_tag = !c.show_name_tag,
                        5 => c.force_elytra_when_flying = !c.force_elytra_when_flying,
                        _ => {
                            let p = c.piece_mut(match i {
                                2 => 1,
                                3 => 0,
                                _ => 2,
                            });
                            if right {
                                p.glint = !p.glint;
                            } else {
                                p.base = !p.base;
                            }
                        }
                    }
                }
            }
            Act::CreateOverride => {
                if let Some(k) = s.selected.clone() {
                    let title = names
                        .iter()
                        .find(|(p, _)| p.as_deref() == Some(&k))
                        .map_or_else(|| if k == own.to_lowercase() { own.clone() } else { k.clone() }, |(_, n)| n.clone());
                    let config = s.cfg.global.clone();
                    s.cfg.overrides.insert(k, Override { title, config });
                }
            }
            Act::DeleteOverride => {
                if let Some(k) = &s.selected {
                    s.cfg.overrides.remove(k);
                }
            }
        }
        save(s);
    });
}

pub(crate) fn screen_drag(game: &Game<'_>, screen: &NativeScreenView<'_>, canceled: &mut bool) {
    if !mine(game) {
        return;
    }
    *canceled = true;
    let Some(drag) = with(|s| s.drag) else { return };
    let [w, h] = with(|s| s.size);
    let lay = layout(game, w, h);
    let want = match drag {
        Drag::Slider(i) => Act::Slider(i),
        Drag::Cooldown => Act::Cooldown,
        Drag::Fade => Act::Fade,
    };
    if let Some(r) = hit_rect(&lay, &want) {
        with(|s| set_slider(s, drag, r, screen.mouse[0]));
    }
}

pub(crate) fn screen_release(game: &Game<'_>, _screen: &NativeScreenView<'_>, canceled: &mut bool) {
    if !mine(game) {
        return;
    }
    *canceled = true;
    with(|s| {
        if s.drag.take().is_some() {
            save(s);
        }
    });
}

pub(crate) fn screen_scroll(game: &Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    if !mine(game) {
        return;
    }
    *canceled = true;
    let n = with(|s| players(game, &s.cfg).len());
    with(|s| {
        let max = (n as f32 * 20.0 - (PANEL_W - 16.0)).max(0.0);
        s.scroll = (s.scroll - scroll[1] as f32 * 20.0).clamp(0.0, max);
    });
}

#[cfg(test)]
pub(crate) fn test_edit(f: impl FnOnce(&mut ModConfig)) {
    with(|s| {
        s.loaded = true;
        f(&mut s.cfg);
    });
}
#[cfg(test)]
pub(crate) fn test_cfg() -> ModConfig {
    with(|s| s.cfg.clone())
}
/// The middle of the helmet slider, at `frac` along it, and of the back button.
#[cfg(test)]
pub(crate) fn test_targets(w: f32, h: f32, g: &Game<'_>, frac: f32) -> ([f32; 2], [f32; 2]) {
    let lay = layout(g, w, h);
    let s = hit_rect(&lay, &Act::Slider(0)).unwrap();
    let back = lay.back;
    ([s[0] + 4.0 + (s[2] - 8.0) * frac, s[1] + s[3] / 2.0], [back[0] + 5.0, back[1] + 5.0])
}
