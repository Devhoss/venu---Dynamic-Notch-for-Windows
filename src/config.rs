use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EdgeSelection {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PaddingConfig {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "Segoe UI".to_string(),
            size: 20.0,
            bold: true,
            italic: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorConfig {
    pub text_color: [f32; 4], // RGBA 0.0 - 1.0
    pub bg_color: [f32; 4],   // RGBA 0.0 - 1.0
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            text_color: [1.0, 0.9, 0.2, 1.0],   // Vibrant Gold / Yellow
            bg_color: [0.08, 0.08, 0.12, 0.85], // Sleek Dark Semi-Transparent
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimConfig {
    pub speed: f32, // pixels per second
    pub reverse: bool,
}

impl Default for AnimConfig {
    fn default() -> Self {
        Self {
            speed: 120.0,
            reverse: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Notch
// ---------------------------------------------------------------------------

/// Horizontal anchor for the notch on its monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotchAlign {
    Left,
    Center,
    Right,
}

fn default_theme() -> NotchTheme {
    NotchTheme::Dark
}

fn default_true() -> bool {
    true
}

/// How the notch is finished.
///
/// This is a surface treatment, not a full re-skin: the accent stays yours and
/// the layout never changes. Only the panel and the type tones move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotchTheme {
    /// Near-black obsidian. The default, and the only one that truly vanishes
    /// into a laptop bezel.
    Dark,
    /// Bone-white panel with dark type, for light desktops.
    Light,
    /// Translucent frosted glass, with the screen behind it blurred through the panel.
    Frosted,
    /// Pure clear see-through glass without backdrop blur.
    Transparent,
    /// Deep soft-diffusion blur with smooth ambient background colors.
    Blurred,
    /// Windows Fluent Acrylic style with rich ambient tint and balanced diffusion.
    Acrylic,
}

impl NotchTheme {
    pub const ALL: [NotchTheme; 6] = [
        NotchTheme::Dark,
        NotchTheme::Light,
        NotchTheme::Frosted,
        NotchTheme::Transparent,
        NotchTheme::Blurred,
        NotchTheme::Acrylic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NotchTheme::Dark => "Dark",
            NotchTheme::Light => "Light",
            NotchTheme::Frosted => "Frosted",
            NotchTheme::Transparent => "Transparent",
            NotchTheme::Blurred => "Blurred",
            NotchTheme::Acrylic => "Acrylic",
        }
    }

    /// The panel colour this theme wants. Applied when the user switches
    /// themes; they are free to tint it afterwards.
    pub fn default_surface(self) -> [f32; 4] {
        match self {
            NotchTheme::Dark => [0.031, 0.031, 0.043, 0.97],
            NotchTheme::Light => [0.965, 0.965, 0.976, 0.97],
            // Low alpha on purpose: the blurred capture behind it supplies
            // most of the body, and this is only the tint on top.
            NotchTheme::Frosted => [0.07, 0.07, 0.09, 0.55],
            NotchTheme::Transparent => [0.04, 0.04, 0.06, 0.28],
            NotchTheme::Blurred => [0.06, 0.06, 0.08, 0.60],
            NotchTheme::Acrylic => [0.09, 0.09, 0.13, 0.70],
        }
    }
}

/// How the settings window is painted.
///
/// `System` follows whatever Windows reports for apps, so a machine set to
/// switch at sunset takes the window with it. The other two pin it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UiTheme {
    #[default]
    System,
    Light,
    Dark,
}

impl UiTheme {
    pub const ALL: [UiTheme; 3] = [UiTheme::System, UiTheme::Light, UiTheme::Dark];

    pub fn label(self) -> &'static str {
        match self {
            UiTheme::System => "System",
            UiTheme::Light => "Light",
            UiTheme::Dark => "Dark",
        }
    }
}

/// The faces the notch can show. The order of [`NotchConfig::slides`] is the
/// carousel order the scroll wheel walks through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideKind {
    /// Split view: what today is about (left) and today's short list (right).
    Status,
    /// Oversized clock plus the date.
    Clock,
    /// The classic scrolling marquee, now living inside the notch.
    Marquee,
    /// A picture you want to be reminded of, with an optional caption.
    Wallpaper,
    /// Whatever is currently playing in the system's media session, if
    /// anything: title, artist, art, and transport controls.
    Media,
    /// Notification Center: alerts, task updates, and messages from allowed apps.
    Notifications,
    /// Claude Code usage: context window, session cost, and rate limits.
    Usage,
}

impl SlideKind {
    pub const ALL: [SlideKind; 7] = [
        SlideKind::Status,
        SlideKind::Clock,
        SlideKind::Marquee,
        SlideKind::Wallpaper,
        SlideKind::Media,
        SlideKind::Notifications,
        SlideKind::Usage,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SlideKind::Status => "Status",
            SlideKind::Clock => "Clock",
            SlideKind::Marquee => "Moving text",
            SlideKind::Wallpaper => "Wallpaper",
            SlideKind::Media => "Now Playing",
            SlideKind::Notifications => "Notifications",
            SlideKind::Usage => "Claude Usage",
        }
    }
}

/// Default face the notch shows when collapsed and resting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CollapsedMode {
    /// Stays on whichever slide was last viewed in the carousel.
    #[default]
    LastActive,
    /// Resets to Status slide.
    Status,
    /// Resets to Clock / Time slide.
    Clock,
    /// Resets to Moving Text marquee.
    Marquee,
    /// Resets to Wallpaper picture slide.
    Wallpaper,
    /// Resets to Now Playing media slide.
    Media,
    /// Resets to Notifications unread indicator.
    Notifications,
    /// Resets to Claude Code usage slide.
    Usage,
    /// Smart / Auto mode: shows active alerts if unread, or Now Playing if music is on, otherwise Clock.
    Auto,
}

impl CollapsedMode {
    pub const ALL: [CollapsedMode; 9] = [
        CollapsedMode::LastActive,
        CollapsedMode::Status,
        CollapsedMode::Clock,
        CollapsedMode::Marquee,
        CollapsedMode::Wallpaper,
        CollapsedMode::Media,
        CollapsedMode::Notifications,
        CollapsedMode::Usage,
        CollapsedMode::Auto,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CollapsedMode::LastActive => "Last Active",
            CollapsedMode::Status => "Status",
            CollapsedMode::Clock => "Clock",
            CollapsedMode::Marquee => "Moving text",
            CollapsedMode::Wallpaper => "Wallpaper",
            CollapsedMode::Media => "Now Playing",
            CollapsedMode::Notifications => "Notifications",
            CollapsedMode::Usage => "Claude Usage",
            CollapsedMode::Auto => "Dynamic / Auto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NotificationGlowStyle {
    #[default]
    CountdownDrain,
    RgbBorderMoving,
    WavyRgbMoving,
    NeonGlow,
}

impl NotificationGlowStyle {
    pub const ALL: [Self; 4] = [
        Self::CountdownDrain,
        Self::RgbBorderMoving,
        Self::WavyRgbMoving,
        Self::NeonGlow,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::CountdownDrain => "Countdown Drain (3-Sided Border)",
            Self::RgbBorderMoving => "Moving RGB Perimeter",
            Self::WavyRgbMoving => "Wavy Moving RGB Wave",
            Self::NeonGlow => "Neon Ambient Bloom",
        }
    }
}

/// What a registered notification source actually is.
///
/// This is metadata for the user, not a permission. `is_app_allowed` never
/// consults it, and neither does webhook matching -- both key off the source
/// name alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SourceKind {
    /// A bare source name chosen by the user, for anything that is not a
    /// Windows executable: agents, CI, scripts, Git hooks.
    #[default]
    Custom,
    /// A real Windows application, whose executable the user picked once by
    /// hand. The path is a label for the user's benefit only.
    WindowsApp,
}

/// One row of the Settings source registry.
///
/// A view over the existing config fields rather than a new store: the
/// registry is still the union of `allowed_apps` and `app_colors`, and an
/// entry present in only one of them is still a row.
#[derive(Debug, Clone, PartialEq)]
pub struct RegistryEntry {
    /// The source name, which is also the webhook `app` identifier.
    pub name: String,
    /// Membership in `allowed_apps` -- whether this source may toast.
    pub allowed: bool,
    /// Whether the user has picked a colour for it yet.
    pub has_color: bool,
    /// Which kind of source this is. Absent from `source_kinds` means Custom.
    pub kind: SourceKind,
    /// The executable the user selected, for Windows-app sources only.
    pub exe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationConfig {
    pub enabled: bool,
    /// Allowed apps whose notifications will trigger the Dynamic Notch alert toast.
    pub allowed_apps: Vec<String>,
    /// How long the dynamic alert capsule dwells on screen before settling back (in seconds).
    pub toast_duration_secs: f32,
    /// Local HTTP webhook server port for receiving notifications from AI tools & scripts.
    pub webhook_port: u16,
    pub sound_enabled: bool,
    /// Glowing border style when notification alert appears.
    pub glow_style: NotificationGlowStyle,
    /// Custom RGBA colors mapped per application name.
    pub app_colors: std::collections::HashMap<String, [f32; 4]>,
    /// Which kind of source each name is. Absent means
    /// [`SourceKind::Custom`], which is what every pre-V2 entry is.
    pub source_kinds: std::collections::HashMap<String, SourceKind>,
    /// The executable each Windows-app source points at, as the user
    /// selected it. Never canonicalized, never revalidated, never used for
    /// matching -- see [`SourceKind`].
    pub source_exes: std::collections::HashMap<String, String>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        let mut app_colors = std::collections::HashMap::new();
        app_colors.insert("Antigravity".to_string(), [0.0, 0.94, 1.0, 1.0]); // #00F0FF Electric Cyan
        app_colors.insert("Codex".to_string(), [0.06, 0.72, 0.51, 1.0]); // #10B981 Emerald
        app_colors.insert("Claude".to_string(), [0.98, 0.45, 0.09, 1.0]); // #EA580C Terracotta
        app_colors.insert("Cursor".to_string(), [0.39, 0.40, 0.95, 1.0]); // #6366F1 Indigo
        app_colors.insert("Terminal".to_string(), [0.66, 0.33, 0.97, 1.0]); // #A855F7 Purple
        app_colors.insert("VS Code".to_string(), [0.0, 0.47, 0.83, 1.0]); // #0078D4 VS Blue
        app_colors.insert("Slack".to_string(), [0.88, 0.12, 0.35, 1.0]); // #E01E5A Berry
        app_colors.insert("Discord".to_string(), [0.35, 0.40, 0.95, 1.0]); // #5865F2 Blurple

        Self {
            enabled: true,
            allowed_apps: vec![
                "Antigravity".to_string(),
                "Codex".to_string(),
                "Claude".to_string(),
                "Cursor".to_string(),
                "Terminal".to_string(),
                "VS Code".to_string(),
            ],
            toast_duration_secs: 4.5,
            webhook_port: 18923,
            sound_enabled: true,
            glow_style: NotificationGlowStyle::CountdownDrain,
            app_colors,
            source_kinds: std::collections::HashMap::new(),
            source_exes: std::collections::HashMap::new(),
        }
    }
}

impl NotificationConfig {
    /// Colour handed to an application that has never had one picked for it.
    /// Matches the `Info` badge colour, so an unconfigured app looks the same
    /// as it always has rather than picking up a new identity here.
    pub const DEFAULT_APP_COLOR: [f32; 4] = [0.22, 0.74, 0.97, 1.0];

    /// Longest application name the registry will store. Long enough for any
    /// real product name, short enough that a hostile webhook cannot bloat
    /// the config with one enormous key.
    pub const MAX_APP_NAME_LEN: usize = 64;

    /// Every source this config knows about, as a row of the Settings view.
    ///
    /// This is the union of the notification allowlist and the colour map,
    /// which is exactly what [`Self::app_registry`] returns -- the same set,
    /// carried into the shape the UI renders. Either list can hold a name the
    /// other does not: a source may be allowed but uncoloured, or coloured
    /// but not allowed. Both belong in Settings, so both are surfaced. Order
    /// is stable -- allowlisted sources first, then colour-only ones -- and
    /// each is deduped case-insensitively.
    pub fn registry_entries(&self) -> Vec<RegistryEntry> {
        self.app_registry()
            .into_iter()
            .map(|name| RegistryEntry {
                kind: self.source_kind(&name),
                exe: self.source_exes.get(&name).cloned(),
                allowed: self.is_app_allowed(&name),
                has_color: self
                    .app_colors
                    .keys()
                    .any(|existing| existing.eq_ignore_ascii_case(&name)),
                name,
            })
            .collect()
    }

    /// Which kind of source this name is. An absent entry means
    /// [`SourceKind::Custom`], which is correct for every source registered
    /// before V2 as well as for any added as one.
    pub fn source_kind(&self, name: &str) -> SourceKind {
        if let Some(kind) = self.source_kinds.get(name) {
            return *kind;
        }
        self.source_kinds
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, kind)| *kind)
            .unwrap_or_default()
    }

    /// Every source this config knows about, as a union of the notification
    /// allowlist and the colour map.
    ///
    /// Either list can hold a name the other does not: an app may be allowed
    /// but uncoloured, or coloured but not allowed. Both belong in Settings,
    /// so both are surfaced. Order is stable -- allowlisted apps first, then
    /// colour-only apps -- and each is deduped case-insensitively.
    pub fn app_registry(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();

        for name in self.allowed_apps.iter().chain(self.app_colors.keys()) {
            if !seen
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(name))
            {
                seen.push(name.clone());
            }
        }

        seen
    }

    /// Whether this app may raise a notification toast. An empty allowlist
    /// means "everything", which is the long-standing behaviour and stays.
    pub fn is_app_allowed(&self, app: &str) -> bool {
        self.allowed_apps.is_empty()
            || self
                .allowed_apps
                .iter()
                .any(|a| a.eq_ignore_ascii_case(app))
    }

    /// Clean up user input and reject names that cannot be stored.
    ///
    /// Returns the name trimmed to its canonical form, or `None` when it is
    /// empty, whitespace-only, or longer than [`Self::MAX_APP_NAME_LEN`].
    /// Display casing is preserved; only surrounding whitespace goes.
    pub fn normalize_app_name(input: &str) -> Option<String> {
        let name = input.trim();
        if name.is_empty() || name.len() > Self::MAX_APP_NAME_LEN {
            return None;
        }
        Some(name.to_string())
    }

    /// Register a new application: allowed to notify, with a default colour.
    ///
    /// Returns `false` when the name is invalid or already registered in any
    /// casing, so the caller can report the rejection rather than silently
    /// adding a near-duplicate. The colour is written eagerly because the
    /// picker needs something to edit and store.
    pub fn add_app(&mut self, input: &str) -> bool {
        let Some(name) = Self::normalize_app_name(input) else {
            return false;
        };

        let already_known = self
            .allowed_apps
            .iter()
            .chain(self.app_colors.keys())
            .any(|existing| existing.eq_ignore_ascii_case(&name));
        if already_known {
            return false;
        }

        self.allowed_apps.push(name.clone());
        self.app_colors.insert(name, Self::DEFAULT_APP_COLOR);
        true
    }

    /// Register a source explicitly, as either kind.
    ///
    /// A custom source writes only the name, which already reads back as
    /// [`SourceKind::Custom`] because that is the fallback -- so there is
    /// nothing to record. A Windows application additionally records its kind
    /// and the executable the user picked. Neither the kind nor the path is
    /// consulted when deciding whether a notification is allowed.
    ///
    /// Returns `false` when the name is invalid or already registered in any
    /// casing, so the caller can report the rejection rather than silently
    /// adding a near-duplicate.
    pub fn add_source(&mut self, input: &str, kind: SourceKind, exe: Option<&str>) -> bool {
        if !self.add_app(input) {
            return false;
        }

        let name = input.trim().to_string();
        if kind == SourceKind::WindowsApp {
            self.source_kinds
                .insert(name.clone(), SourceKind::WindowsApp);
            // Stored exactly as chosen: not canonicalized, and never checked
            // again. A moved executable does not stop the source working,
            // because delivery depends on the name alone.
            if let Some(exe) = exe {
                self.source_exes.insert(name, exe.to_string());
            }
        }

        true
    }

    /// Drop a source from every structure it appears in: the allowlist, the
    /// colour map, the kind map and the executable map. Any of them may be
    /// the only place it appears, so all are cleared. No other source is
    /// touched.
    pub fn remove_app(&mut self, app: &str) -> bool {
        let before = self.allowed_apps.len() + self.app_colors.len();

        self.allowed_apps
            .retain(|existing| !existing.eq_ignore_ascii_case(app));

        self.app_colors
            .retain(|existing, _| !existing.eq_ignore_ascii_case(app));

        self.source_kinds
            .retain(|existing, _| !existing.eq_ignore_ascii_case(app));

        self.source_exes
            .retain(|existing, _| !existing.eq_ignore_ascii_case(app));

        before
            != self.allowed_apps.len()
                + self.app_colors.len()
                + self.source_kinds.len()
                + self.source_exes.len()
    }

    /// Allow or disallow an application, leaving its colour alone.
    ///
    /// Disallowing keeps the name in the registry -- it just stops toasting --
    /// so the user can switch it back on without re-adding it.
    pub fn set_app_allowed(&mut self, app: &str, allowed: bool) {
        let present = self
            .allowed_apps
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(app));

        match (allowed, present) {
            (true, false) => self.allowed_apps.push(app.trim().to_string()),
            (false, true) => self
                .allowed_apps
                .retain(|existing| !existing.eq_ignore_ascii_case(app)),
            // An empty allowlist is the long-standing "everything is allowed"
            // shorthand, so there is no entry to remove. Turning one app off
            // then has to become explicit: name every other app first, or the
            // toggle would read as doing nothing.
            (false, false) if self.allowed_apps.is_empty() => {
                let others: Vec<String> = self
                    .app_registry()
                    .into_iter()
                    .filter(|name| !name.eq_ignore_ascii_case(app))
                    .collect();
                self.allowed_apps = others;
            }
            _ => {}
        }
    }

    pub fn get_app_color(&self, app: &str) -> [f32; 4] {
        if let Some(c) = self.app_colors.get(app) {
            return *c;
        }
        for (k, v) in &self.app_colors {
            if k.eq_ignore_ascii_case(app) {
                return *v;
            }
        }
        Self::DEFAULT_APP_COLOR
    }
}

/// One line on today's short list. Deliberately not a to-do system: this is
/// only meant to hold the handful of things today is actually about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusItem {
    pub text: String,
    pub done: bool,
}

impl StatusItem {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            done: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusConfig {
    /// Small tracked label above the focus line, e.g. "TODAY".
    pub heading: String,
    /// The one thing being worked on right now. Editable inline from the notch.
    pub focus: String,
    /// Everything else today is about. Capped in the UI, not here.
    pub items: Vec<StatusItem>,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            heading: "TODAY".to_string(),
            focus: "Set what you are working on".to_string(),
            items: vec![
                StatusItem::new("Ship the notch overlay"),
                StatusItem::new("Review pull requests"),
                StatusItem::new("Write the release notes"),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallpaperConfig {
    /// Absolute path to a PNG/JPG/BMP/GIF. Empty means "show the placeholder".
    pub path: String,
    pub caption: String,

    /// Which point of the image to keep in frame when it is cropped to the
    /// panel, as a 0..1 fraction of the image. 0.5/0.5 is dead centre.
    #[serde(default = "half")]
    pub focus_x: f32,
    #[serde(default = "half")]
    pub focus_y: f32,

    /// Extra magnification on top of the cover fit. 1.0 shows as much of the
    /// image as the panel shape allows.
    #[serde(default = "one")]
    pub zoom: f32,

    /// Panel size while the wallpaper slide is showing. Zero means "use the
    /// notch's normal expanded size"; a picture usually wants more room than a
    /// line of text does, so it gets to ask for its own.
    #[serde(default)]
    pub panel_width: u32,
    #[serde(default)]
    pub panel_height: u32,
}

fn half() -> f32 {
    0.5
}

fn one() -> f32 {
    1.0
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            caption: String::new(),
            focus_x: 0.5,
            focus_y: 0.5,
            zoom: 1.0,
            panel_width: 0,
            panel_height: 0,
        }
    }
}

impl WallpaperConfig {
    pub fn has_image(&self) -> bool {
        !self.path.trim().is_empty()
    }

    /// Clamp the framing controls into ranges the painter can rely on.
    pub fn sanitised(&self) -> (f32, f32, f32) {
        (
            self.focus_x.clamp(0.0, 1.0),
            self.focus_y.clamp(0.0, 1.0),
            self.zoom.clamp(1.0, 4.0),
        )
    }
}

/// Per-slide overrides for the moving-text slide.
///
/// A scrolling line wants a different shape than a clock does — more width,
/// less height, and its own type size. Every field is an override rather than
/// a value: `0` means "whatever the notch itself is set to", so the slide only
/// diverges where the user deliberately made it diverge, and the notch's own
/// settings keep working as the single place to change everything at once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MarqueeConfig {
    /// Whether the line scrolls. Off, it simply sits there: the same message,
    /// held still. Motion is what makes a marquee readable at a glance from
    /// across the room, and also what makes it impossible to ignore while you
    /// are trying to work — so which one you want depends on the message.
    pub scroll: bool,
    pub collapsed_width: u32,
    pub collapsed_height: u32,
    pub panel_width: u32,
    pub panel_height: u32,
    /// Type size of the line in the open panel.
    pub font_size: f32,
    /// Type size of the line in the collapsed pill.
    pub pill_font_size: f32,
}

impl Default for MarqueeConfig {
    fn default() -> Self {
        Self {
            // Scrolling is the whole point of the slide; a static line is the
            // deliberate choice, not the starting state.
            scroll: true,
            collapsed_width: 0,
            collapsed_height: 0,
            panel_width: 0,
            panel_height: 0,
            font_size: 0.0,
            pill_font_size: 0.0,
        }
    }
}

impl MarqueeConfig {
    /// Type size for the open panel, or `fallback` when the user has not
    /// asked for one. Clamped because the value round-trips through JSON and
    /// a hand-edited zero-or-huge size would otherwise reach DirectWrite.
    pub fn panel_font(&self, fallback: f32) -> f32 {
        if self.font_size > 0.0 {
            self.font_size.clamp(8.0, 120.0)
        } else {
            fallback
        }
    }

    /// Type size for the collapsed pill, or `fallback`.
    pub fn pill_font(&self, fallback: f32) -> f32 {
        if self.pill_font_size > 0.0 {
            self.pill_font_size.clamp(6.0, 64.0)
        } else {
            fallback
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotchConfig {
    pub enabled: bool,
    pub monitor_index: usize,
    pub align: NotchAlign,
    /// Nudge from the anchor, in pixels. Positive is right.
    pub offset_x: i32,
    /// Distance from the top of the monitor. 0 keeps the notch fused to the
    /// bezel and enables the concave shoulders; anything else detaches it.
    pub offset_y: i32,
    pub collapsed_width: u32,
    pub collapsed_height: u32,
    pub expanded_width: u32,
    pub expanded_height: u32,
    pub slides: Vec<SlideKind>,
    /// The slide the notch rests on when collapsed. Persisted so the notch
    /// comes back showing whatever you last left it on.
    pub active_slide: usize,
    #[serde(default)]
    pub default_collapsed: CollapsedMode,
    #[serde(default)]
    pub notifications: NotificationConfig,
    pub accent: [f32; 4],
    pub surface: [f32; 4],
    #[serde(default = "default_theme")]
    pub theme: NotchTheme,
    pub font_family: String,
    pub clock_24h: bool,
    /// Grace period after the cursor leaves before collapsing, so brushing
    /// past the edge of the panel does not slam it shut.
    pub collapse_delay_ms: u32,
    pub scroll_to_switch: bool,
    pub always_on_top: bool,
    /// When on, the notch stops taking mouse clicks: they land on whatever is
    /// underneath instead. It still opens on hover and still answers the
    /// wheel, so the deck stays usable — only clicking through to the window
    /// below changes.
    ///
    /// Toggled by pressing the left and right mouse buttons together with the
    /// cursor over the notch, because once click-through is on there is no
    /// button left to press to turn it back off.
    #[serde(default)]
    pub click_through: bool,
}

impl Default for NotchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monitor_index: 0,
            align: NotchAlign::Center,
            offset_x: 0,
            offset_y: 0,
            collapsed_width: 188,
            collapsed_height: 34,
            expanded_width: 760,
            expanded_height: 208,
            slides: SlideKind::ALL.to_vec(),
            active_slide: 0,
            default_collapsed: CollapsedMode::LastActive,
            notifications: NotificationConfig::default(),
            accent: [1.0, 0.604, 0.235, 1.0],     // ember
            surface: [0.031, 0.031, 0.043, 0.97], // obsidian
            theme: NotchTheme::Dark,
            font_family: "Plus Jakarta Sans".to_string(),
            clock_24h: false,
            collapse_delay_ms: 220,
            scroll_to_switch: true,
            always_on_top: true,
            click_through: false,
        }
    }
}

impl NotchConfig {
    /// Slides, guaranteed non-empty, so the carousel always has something to
    /// land on even if every slide was unchecked in settings.
    /// The deck as it is actually shown.
    ///
    /// Borrowed rather than cloned: this is read several times per frame — by
    /// the placement maths, the carousel blend and the painter — and handing
    /// back a fresh `Vec` each time put a heap allocation on every one of
    /// those calls, sixty times a second, for a list that almost never
    /// changes.
    pub fn effective_slides(&self) -> &[SlideKind] {
        const FALLBACK: [SlideKind; 1] = [SlideKind::Clock];
        if self.slides.is_empty() {
            &FALLBACK
        } else {
            &self.slides
        }
    }

    pub fn clamped_active(&self) -> usize {
        let len = self.effective_slides().len();
        self.active_slide.min(len.saturating_sub(1))
    }
}

// ---------------------------------------------------------------------------
// FlashScreen
// ---------------------------------------------------------------------------

/// What the periodic flash puts on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlashContent {
    #[default]
    Text,
    Image,
    /// The image and the text share one flash, side by side or stacked.
    Both,
}

impl FlashContent {
    pub const ALL: [FlashContent; 3] =
        [FlashContent::Text, FlashContent::Image, FlashContent::Both];

    pub fn label(self) -> &'static str {
        match self {
            FlashContent::Text => "Text",
            FlashContent::Image => "Image",
            FlashContent::Both => "Text + Image",
        }
    }
}

/// How the text sits against the image when the flash carries both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlashLayout {
    #[default]
    TextTop,
    TextBottom,
    TextLeft,
    TextRight,
}

impl FlashLayout {
    pub const ALL: [FlashLayout; 4] = [
        FlashLayout::TextTop,
        FlashLayout::TextBottom,
        FlashLayout::TextLeft,
        FlashLayout::TextRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FlashLayout::TextTop => "Text on top",
            FlashLayout::TextBottom => "Text below",
            FlashLayout::TextLeft => "Text on left",
            FlashLayout::TextRight => "Text on right",
        }
    }
}

/// What sits behind the content block. Drawn only behind the message itself,
/// never across the whole screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlashBackground {
    #[default]
    Transparent,
    /// Blurred desktop behind the block, plus a light wash.
    Frosted,
    /// Flat white tint.
    White,
    /// Flat dark tint.
    Dark,
    /// Blurred desktop behind the block, no tint.
    Blur,
}

impl FlashBackground {
    pub const ALL: [FlashBackground; 5] = [
        FlashBackground::Transparent,
        FlashBackground::Frosted,
        FlashBackground::White,
        FlashBackground::Dark,
        FlashBackground::Blur,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FlashBackground::Transparent => "Transparent",
            FlashBackground::Frosted => "Frosted",
            FlashBackground::White => "White overlay",
            FlashBackground::Dark => "Dark overlay",
            FlashBackground::Blur => "Blur",
        }
    }

    /// Frosted and Blur sample the screen behind the window, which needs the
    /// window kept out of its own capture.
    pub fn samples_desktop(self) -> bool {
        matches!(self, FlashBackground::Frosted | FlashBackground::Blur)
    }
}

/// How each flash arrives and leaves. One choice covers both halves: a slide
/// that enters from one edge exits through the other, and the zoom that
/// reveals from the centre folds back into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FlashAnim {
    /// Reveals with a zoom in from the centre, leaves with a fade + zoom out.
    #[default]
    ZoomCenter,
    /// Sweeps in from the left edge, settles in the centre, exits to the right.
    SlideLeftToRight,
    /// Sweeps in from the right edge, settles in the centre, exits to the left.
    SlideRightToLeft,
    /// Plain opacity fade in and out, no movement.
    Fade,
}

impl FlashAnim {
    pub const ALL: [FlashAnim; 4] = [
        FlashAnim::ZoomCenter,
        FlashAnim::SlideLeftToRight,
        FlashAnim::SlideRightToLeft,
        FlashAnim::Fade,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FlashAnim::ZoomCenter => "Zoom reveal from center",
            FlashAnim::SlideLeftToRight => "Slide: left to right",
            FlashAnim::SlideRightToLeft => "Slide: right to left",
            FlashAnim::Fade => "Fade: soft in, soft out",
        }
    }
}

/// The FlashScreen: every so often, for a few seconds, a line of text, an
/// image, or both appears in the middle of the screen — then leaves on its
/// own. Everything that can hold more than one entry rotates: each flash uses
/// the next text, the next image, and the next colour in turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FlashConfig {
    pub enabled: bool,
    /// Seconds between the end of one flash and the start of the next.
    pub interval_secs: f32,
    /// How long the flash stays at full presence, before its exit begins.
    pub duration_secs: f32,
    pub content: FlashContent,
    /// Only meaningful when the content is Both.
    pub layout: FlashLayout,
    /// The messages, rotated one per flash.
    pub texts: Vec<String>,
    /// Image paths, rotated one per flash.
    pub images: Vec<String>,
    pub font: FontConfig,
    /// Text colours, rotated one per flash — or swept as a gradient.
    pub text_colors: Vec<[f32; 4]>,
    /// Paint the text with a gradient of every colour instead of one solid
    /// colour per flash.
    pub gradient_text: bool,
    /// A glare sweeping left to right across the text, once per flash.
    pub shine: bool,
    /// How long that one pass takes, in seconds.
    pub shine_speed_secs: f32,
    /// Image height as a fraction of the screen height.
    pub image_scale: f32,
    /// What is drawn behind the content block.
    pub bg_kind: FlashBackground,
    /// 0 (invisible) to 1 (full effect) — the strength of the background.
    pub bg_strength: f32,
    /// How far the background extends beyond the content, in pixels, on
    /// every side.
    pub bg_padding: f32,
    pub anim: FlashAnim,
    /// Length of the entry/exit animation. Short is snappy, long is smooth.
    pub anim_speed_secs: f32,
    pub monitor_index: usize,
    pub click_through: bool,
    pub always_on_top: bool,
}

impl Default for FlashConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: 300.0,
            duration_secs: 5.0,
            content: FlashContent::Text,
            layout: FlashLayout::TextTop,
            texts: vec!["⚡ Stay focused!".to_string()],
            images: Vec::new(),
            font: FontConfig {
                family: "Segoe UI".to_string(),
                size: 96.0,
                bold: true,
                italic: false,
            },
            text_colors: vec![[1.0, 1.0, 1.0, 1.0]],
            gradient_text: false,
            shine: true,
            shine_speed_secs: 1.2,
            image_scale: 0.5,
            bg_kind: FlashBackground::Transparent,
            bg_strength: 0.6,
            bg_padding: 48.0,
            anim: FlashAnim::ZoomCenter,
            anim_speed_secs: 0.6,
            monitor_index: 0,
            click_through: true,
            always_on_top: true,
        }
    }
}

/// What a flash shows when the message list holds only blank lines — a flash
/// of nothing would look like the feature is broken.
pub const DEFAULT_FLASH_TEXT: &str = "⚡ Stay focused!";

impl FlashConfig {
    /// Intervals round-trip through JSON, so a hand-edited zero or negative
    /// must not reach the frame loop.
    pub fn safe_interval(&self) -> f32 {
        self.interval_secs.max(5.0)
    }

    pub fn safe_duration(&self) -> f32 {
        self.duration_secs.clamp(0.5, 600.0)
    }

    pub fn safe_anim_secs(&self) -> f32 {
        self.anim_speed_secs.clamp(0.1, 3.0)
    }

    pub fn safe_shine_secs(&self) -> f32 {
        self.shine_speed_secs.clamp(0.2, 8.0)
    }

    /// The message this turn shows. `turn` is the flash counter, so the list
    /// walks forward one entry per flash.
    pub fn text_for_turn(&self, turn: u32) -> &str {
        if self.texts.is_empty() {
            return DEFAULT_FLASH_TEXT;
        }
        let text = &self.texts[turn as usize % self.texts.len()];
        if text.trim().is_empty() {
            DEFAULT_FLASH_TEXT
        } else {
            text
        }
    }

    /// The image this turn shows. Empty when there is nothing to show.
    pub fn image_for_turn(&self, turn: u32) -> &str {
        if self.images.is_empty() {
            ""
        } else {
            self.images[turn as usize % self.images.len()].trim()
        }
    }

    /// The colour this turn's text uses, rotating through the list.
    pub fn color_for_turn(&self, turn: u32) -> [f32; 4] {
        if self.text_colors.is_empty() {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            self.text_colors[turn as usize % self.text_colors.len()]
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub text: String,
    /// Master switch for the edge marquee — the strips of scrolling text
    /// around the screen border, which are a separate overlay from the notch.
    /// Off, every strip is torn down regardless of which edges are ticked, so
    /// the edge selection is preserved for when it comes back.
    #[serde(default = "default_true")]
    pub overlay_enabled: bool,
    #[serde(default = "default_phrase_spacing")]
    pub phrase_spacing: u32,
    pub edges: EdgeSelection,
    pub padding: PaddingConfig,
    pub thickness: u32,
    pub font: FontConfig,
    pub colors: ColorConfig,
    pub animation: AnimConfig,
    pub click_through: bool,
    pub always_on_top: bool,
    pub monitor_index: usize,
    #[serde(default)]
    pub notch: NotchConfig,
    #[serde(default)]
    pub status: StatusConfig,
    #[serde(default)]
    pub wallpaper: WallpaperConfig,
    #[serde(default)]
    pub marquee: MarqueeConfig,
    #[serde(default)]
    pub flash: FlashConfig,
    /// How the settings window itself is painted. Nothing to do with the
    /// overlays — this is only the chrome around the controls.
    #[serde(default)]
    pub ui_theme: UiTheme,
    /// Whether Venu registers itself to start when Windows signs in. The
    /// registry side of this lives in `crate::autostart`; the entry is
    /// refreshed against this flag on every launch.
    #[serde(default)]
    pub launch_on_startup: bool,
}

fn default_phrase_spacing() -> u32 {
    6
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            text: "⭐ REMINDER: Stay focused & stay hydrated! • Venu Dynamic Notch ⭐".to_string(),
            overlay_enabled: true,
            phrase_spacing: 6,
            edges: EdgeSelection::default(),
            padding: PaddingConfig::default(),
            thickness: 36,
            font: FontConfig::default(),
            colors: ColorConfig::default(),
            animation: AnimConfig::default(),
            click_through: true,
            always_on_top: true,
            monitor_index: 0,
            notch: NotchConfig::default(),
            status: StatusConfig::default(),
            wallpaper: WallpaperConfig::default(),
            marquee: MarqueeConfig::default(),
            flash: FlashConfig::default(),
            ui_theme: UiTheme::default(),
            launch_on_startup: false,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(mut path) = dirs::config_dir() {
            path.push("venu");
            let _ = fs::create_dir_all(&path);
            path.push("config.json");
            path
        } else {
            PathBuf::from("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    return config;
                }
            }
        }

        // Backward compatibility fallback: check legacy movingtext path if venu doesn't exist yet
        if let Some(mut legacy_path) = dirs::config_dir() {
            legacy_path.push("movingtext");
            legacy_path.push("config.json");
            if legacy_path.exists() {
                if let Ok(content) = fs::read_to_string(&legacy_path) {
                    if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                        config.save();
                        return config;
                    }
                }
            }
        }

        let default_cfg = Self::default();
        default_cfg.save();
        default_cfg
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }
}

#[cfg(test)]
mod notification_registry_tests {
    use super::*;

    fn has(cfg: &NotificationConfig, name: &str) -> bool {
        cfg.app_registry()
            .iter()
            .any(|a| a.eq_ignore_ascii_case(name))
    }

    #[test]
    fn registry_unions_allowlist_and_colors() {
        let mut cfg = NotificationConfig::default();
        // A colour-only app: not allowed, but should still be visible.
        cfg.app_colors
            .insert("ColorOnly".to_string(), [1.0, 0.0, 0.0, 1.0]);
        // An allowlist-only app: allowed, never coloured.
        cfg.allowed_apps.push("AllowOnly".to_string());

        let names = cfg.app_registry();

        assert!(names.contains(&"ColorOnly".to_string()));
        assert!(names.contains(&"AllowOnly".to_string()));
        for preset in ["Antigravity", "Codex", "Claude", "Discord"] {
            assert!(
                has(&cfg, preset),
                "default app {preset} missing from registry"
            );
        }
    }

    #[test]
    fn add_app_registers_in_both_lists() {
        let mut cfg = NotificationConfig::default();
        assert!(cfg.add_app("Hermes"));

        assert!(cfg.allowed_apps.iter().any(|a| a == "Hermes"));
        assert!(cfg
            .app_colors
            .get("Hermes")
            .copied()
            .is_some_and(|c| c == NotificationConfig::DEFAULT_APP_COLOR));
        assert!(cfg.is_app_allowed("Hermes"));
    }

    #[test]
    fn add_app_rejects_duplicates_and_bad_input() {
        let mut cfg = NotificationConfig::default();
        assert!(cfg.add_app("Hermes"));

        // Same app, different casing, must not be added twice.
        assert!(!cfg.add_app("hermes"));
        assert!(!cfg.add_app("HERMES"));
        assert_eq!(
            cfg.allowed_apps
                .iter()
                .filter(|a| a.eq_ignore_ascii_case("Hermes"))
                .count(),
            1
        );

        // Names that cannot be stored.
        assert!(!cfg.add_app(""));
        assert!(!cfg.add_app("   "));
        assert!(!cfg.add_app(&"x".repeat(NotificationConfig::MAX_APP_NAME_LEN + 1)));

        // An existing default app is a duplicate, not a new entry.
        assert!(!cfg.add_app("discord"));
    }

    #[test]
    fn add_app_preserves_display_casing() {
        let mut cfg = NotificationConfig::default();
        assert!(cfg.add_app("  OpenClaw  "));
        assert!(cfg.allowed_apps.iter().any(|a| a == "OpenClaw"));
    }

    #[test]
    fn remove_app_clears_both_lists() {
        let mut cfg = NotificationConfig::default();
        cfg.add_app("Hermes");
        assert!(cfg.remove_app("hermes"));

        assert!(!cfg.allowed_apps.iter().any(|a| a == "Hermes"));
        assert!(!cfg.app_colors.contains_key("Hermes"));
        assert!(!has(&cfg, "Hermes"));

        // Built-ins survive the removal.
        assert!(has(&cfg, "Claude"));
        // Removing something absent is a no-op, not a silent success.
        assert!(!cfg.remove_app("NeverExisted"));
    }

    #[test]
    fn toggling_permission_keeps_the_app_registered() {
        let mut cfg = NotificationConfig::default();
        cfg.add_app("Hermes");

        cfg.set_app_allowed("Hermes", false);
        assert!(!cfg.is_app_allowed("Hermes"));
        // Still known, so it can be switched back on without re-adding.
        assert!(has(&cfg, "Hermes"));
        assert!(cfg.app_colors.contains_key("Hermes"));

        cfg.set_app_allowed("Hermes", true);
        assert!(cfg.is_app_allowed("Hermes"));
    }

    #[test]
    fn empty_allowlist_still_means_everything() {
        let cfg = NotificationConfig::default();
        assert!(cfg.is_app_allowed("Antigravity"));

        let mut open = NotificationConfig::default();
        open.allowed_apps.clear();
        assert!(open.is_app_allowed("AnythingAtAll"));
    }

    #[test]
    fn disabling_from_wildcard_allowlist_becomes_explicit() {
        // An empty allowlist means "all allowed", so there is no entry to
        // remove. The toggle still has to take effect, which means naming
        // every other app instead of silently doing nothing.
        let mut cfg = NotificationConfig::default();
        cfg.allowed_apps.clear();
        assert!(cfg.is_app_allowed("Claude"));

        cfg.set_app_allowed("Claude", false);

        assert!(!cfg.is_app_allowed("Claude"));
        assert!(
            cfg.is_app_allowed("Codex"),
            "other apps should stay allowed"
        );
        // The app is still registered, so it can be switched back on.
        assert!(has(&cfg, "Claude"));
    }

    #[test]
    fn uncoloured_allowed_app_falls_back_and_is_editable() {
        let mut cfg = NotificationConfig::default();
        cfg.add_app("Hermes");
        // Simulate a config written before this app had a colour.
        cfg.app_colors.remove("Hermes");

        let fallback = cfg.get_app_color("Hermes");
        assert_eq!(fallback, NotificationConfig::DEFAULT_APP_COLOR);
        assert!(has(&cfg, "Hermes"));

        // And the fallback is storable, which is what the picker needs.
        cfg.app_colors
            .insert("Hermes".to_string(), [0.5, 0.1, 0.9, 1.0]);
        assert_eq!(cfg.get_app_color("Hermes"), [0.5, 0.1, 0.9, 1.0]);
    }

    #[test]
    fn legacy_config_without_new_fields_still_loads() {
        // A config.json written before the registry existed: the two fields
        // the registry is built from, and nothing else.
        let legacy = r#"{
            "enabled": true,
            "allowed_apps": ["Claude", "Custom App"],
            "toast_duration_secs": 4.5,
            "webhook_port": 18923,
            "sound_enabled": true,
            "glow_style": "CountdownDrain",
            "app_colors": {"Custom App": [0.1, 0.2, 0.3, 1.0]}
        }"#;

        let cfg: NotificationConfig = serde_json::from_str(legacy).expect("legacy config parses");

        assert!(cfg.is_app_allowed("Custom App"));
        assert_eq!(cfg.get_app_color("Custom App"), [0.1, 0.2, 0.3, 1.0]);
        // Present in the allowlist but never coloured -> fallback, still listed.
        assert!(has(&cfg, "Claude"));
        assert_eq!(
            cfg.get_app_color("Claude"),
            NotificationConfig::DEFAULT_APP_COLOR
        );
    }

    #[test]
    fn every_default_app_still_has_its_own_colour() {
        // The hard-coded colour match was removed on the strength of this:
        // the seeded map must cover every built-in.
        let cfg = NotificationConfig::default();
        let distinct: Vec<[f32; 4]> = cfg
            .app_registry()
            .iter()
            .map(|a| cfg.get_app_color(a))
            .collect();

        for (name, color) in cfg.app_colors.iter() {
            assert!(
                distinct.contains(color),
                "built-in {name} lost its colour after removing the match fallback"
            );
        }
        // Built-ins are not all the same colour, i.e. the map really is used.
        let mut unique = distinct.clone();
        unique.sort_by(|a, b| a.partial_cmp(b).unwrap());
        unique.dedup();
        assert!(unique.len() > 1);
    }

    #[test]
    fn registry_round_trips_through_serde() {
        let mut cfg = NotificationConfig::default();
        cfg.add_app("Hermes");
        cfg.app_colors
            .insert("Hermes".to_string(), [0.9, 0.1, 0.1, 1.0]);

        let text = serde_json::to_string_pretty(&cfg).unwrap();
        let back: NotificationConfig = serde_json::from_str(&text).unwrap();

        assert!(back.is_app_allowed("Hermes"));
        assert_eq!(back.get_app_color("Hermes"), [0.9, 0.1, 0.1, 1.0]);
        assert!(has(&back, "Hermes"));
    }
}

#[cfg(test)]
mod notification_source_kind_tests {
    use super::*;

    fn entry<'a>(entries: &'a [RegistryEntry], name: &str) -> &'a RegistryEntry {
        entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("no registry entry for {name}"))
    }

    /// A V1-shaped config: the two fields that existed before V2, and nothing
    /// else. This is what every pre-V2 user has on disk.
    fn v1_shaped() -> NotificationConfig {
        let mut cfg = NotificationConfig::default();
        cfg.allowed_apps = vec!["Hermes".to_string(), "CI".to_string()];
        cfg
    }

    #[test]
    fn absent_source_kinds_reads_as_custom() {
        let cfg = v1_shaped();

        // No `source_kinds` was ever written for these names.
        assert!(cfg.source_kinds.is_empty());

        for e in cfg.registry_entries() {
            assert_eq!(
                e.kind,
                SourceKind::Custom,
                "{} should default to Custom",
                e.name
            );
            assert_eq!(cfg.source_kind(&e.name), SourceKind::Custom);
        }
    }

    #[test]
    fn absent_source_exes_reports_no_executable() {
        let cfg = v1_shaped();

        assert!(cfg.source_exes.is_empty());
        for e in cfg.registry_entries() {
            assert!(e.exe.is_none(), "{} should have no exe", e.name);
        }
    }

    #[test]
    fn adding_windows_app_writes_kind_exe_allowlist_and_color() {
        let mut cfg = NotificationConfig::default();

        assert!(cfg.add_source(
            "Wavesurf",
            SourceKind::WindowsApp,
            Some(r"E:\Apps\Wavesurf\wavesurf.exe")
        ));

        // All four structures the design promises, in one call.
        assert!(cfg.allowed_apps.iter().any(|a| a == "Wavesurf"));
        assert_eq!(
            cfg.get_app_color("Wavesurf"),
            NotificationConfig::DEFAULT_APP_COLOR
        );
        assert_eq!(cfg.source_kind("Wavesurf"), SourceKind::WindowsApp);
        assert_eq!(
            cfg.source_exes.get("Wavesurf").map(String::as_str),
            Some(r"E:\Apps\Wavesurf\wavesurf.exe")
        );

        let entries = cfg.registry_entries();
        let e = entry(&entries, "Wavesurf");
        assert!(e.allowed && e.has_color);
        assert_eq!(e.kind, SourceKind::WindowsApp);
    }

    #[test]
    fn adding_custom_leaves_source_kinds_absent() {
        let mut cfg = NotificationConfig::default();

        assert!(cfg.add_source("Hermes2", SourceKind::Custom, None));

        // Custom is the fallback, so nothing needs writing -- which is what
        // keeps a custom add identical to the V1 code path.
        assert!(!cfg.source_kinds.contains_key("Hermes2"));
        assert_eq!(cfg.source_kind("Hermes2"), SourceKind::Custom);
        assert!(cfg.source_exes.is_empty());
        assert!(cfg.allowed_apps.iter().any(|a| a == "Hermes2"));
        assert_eq!(
            cfg.get_app_color("Hermes2"),
            NotificationConfig::DEFAULT_APP_COLOR
        );
    }

    #[test]
    fn remove_clears_kind_and_exe() {
        let mut cfg = NotificationConfig::default();
        cfg.add_source("Wavesurf", SourceKind::WindowsApp, Some(r"C:\w.exe"));
        cfg.add_source("Hermes", SourceKind::Custom, None);

        assert!(cfg.remove_app("Wavesurf"));

        // Gone from all four structures...
        assert!(!cfg.allowed_apps.iter().any(|a| a == "Wavesurf"));
        assert!(!cfg.app_colors.contains_key("Wavesurf"));
        assert!(!cfg.source_kinds.contains_key("Wavesurf"));
        assert!(!cfg.source_exes.contains_key("Wavesurf"));

        // ...and the other source is untouched.
        assert!(cfg.allowed_apps.iter().any(|a| a == "Hermes"));
        assert_eq!(cfg.source_kind("Hermes"), SourceKind::Custom);
    }

    #[test]
    fn old_v1_config_still_loads() {
        // Exactly the JSON a pre-V2 install would have on disk: no
        // source_kinds, no source_exes.
        let raw = r#"{
            "enabled": true,
            "allowed_apps": ["VS Code", "Hermes"],
            "toast_duration_secs": 4.0,
            "webhook_port": 18923,
            "sound_enabled": true,
            "glow_style": "CountdownDrain",
            "app_colors": {"VS Code": [0.0, 0.47, 0.83, 1.0], "Hermes": [0.9, 0.1, 0.1, 1.0]}
        }"#;

        let cfg: NotificationConfig = serde_json::from_str(raw).expect("V1 config must load");

        assert!(cfg.source_kinds.is_empty());
        assert!(cfg.source_exes.is_empty());
        assert_eq!(cfg.source_kind("Hermes"), SourceKind::Custom);
        assert!(cfg.is_app_allowed("Hermes"));
        assert_eq!(cfg.get_app_color("Hermes"), [0.9, 0.1, 0.1, 1.0]);

        // And it survives a round trip without gaining anything.
        let again: NotificationConfig =
            serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(again.source_kind("VS Code"), SourceKind::Custom);
    }

    #[test]
    fn exe_path_is_not_consulted_by_is_app_allowed() {
        let mut cfg = NotificationConfig::default();

        // One source with an executable, one without, otherwise identical.
        assert!(cfg.add_source("Wavesurf", SourceKind::WindowsApp, Some(r"C:\gone.exe")));
        assert!(cfg.add_source("Hermes", SourceKind::Custom, None));

        // The path deliberately points at a file that does not exist. It is
        // never opened, so the permission is identical to a custom source's.
        assert!(cfg.is_app_allowed("Wavesurf"));
        assert!(cfg.is_app_allowed("Hermes"));

        // Disabling works the same way for both kinds, and does not
        // disturb the other source: the allowlist holds both names, so
        // dropping one leaves the other allowed.
        cfg.set_app_allowed("Wavesurf", false);
        assert!(!cfg.is_app_allowed("Wavesurf"));
        assert!(
            cfg.is_app_allowed("Hermes"),
            "disabling one source must not affect another"
        );

        // Dropping the metadata changes nothing about permission either.
        cfg.source_kinds.clear();
        cfg.source_exes.clear();
        assert!(cfg.is_app_allowed("Hermes"));
    }

    #[test]
    fn duplicate_names_differing_by_case_are_rejected() {
        let mut cfg = NotificationConfig::default();

        assert!(cfg.add_source("Wavesurf", SourceKind::WindowsApp, Some(r"C:\w.exe")));
        assert!(!cfg.add_source("wavesurf", SourceKind::WindowsApp, Some(r"C:\other.exe")));
        assert!(!cfg.add_source("WAVESURF", SourceKind::Custom, None));

        // Exactly one entry, and the first executable won.
        let matching = cfg
            .app_registry()
            .iter()
            .filter(|n| n.eq_ignore_ascii_case("wavesurf"))
            .count();
        assert_eq!(matching, 1);
        assert_eq!(
            cfg.source_exes.get("Wavesurf").map(String::as_str),
            Some(r"C:\w.exe")
        );
    }

    #[test]
    fn registry_still_unions_allowlist_and_colors() {
        let mut cfg = NotificationConfig::default();

        // Present in allowed_apps but not app_colors.
        assert!(cfg.add_source("OnlyAllowed", SourceKind::Custom, None));
        cfg.app_colors.remove("OnlyAllowed");

        // Present in app_colors but not allowed_apps.
        cfg.app_colors
            .insert("OnlyColoured".to_string(), [1.0, 0.0, 0.0, 1.0]);

        let entries = cfg.registry_entries();
        let a = entry(&entries, "OnlyAllowed");
        assert!(a.allowed && !a.has_color);
        let c = entry(&entries, "OnlyColoured");
        assert!(!c.allowed && c.has_color);

        // Same set the V1 view produced, not a narrower one.
        assert_eq!(
            entries.len(),
            cfg.app_registry().len(),
            "the view must cover exactly the union"
        );
    }
}
