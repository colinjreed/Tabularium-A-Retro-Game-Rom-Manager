//! Retro colour themes: palettes, fonts, and painted effects (graded
//! panels, scanlines, LCD overlay, circuit board, HUD frames).

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{
    Color32, ColorImage, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId,
    LayerId, Painter, Pos2, Rect, RichText, Shape, Stroke, TextStyle, TextureHandle,
    TextureOptions, Ui, Vec2,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeId {
    #[default]
    Standard,
    GameBoy,
    Nes,
    Commodore64,
    ArcadeCrt,
    VirtualBoy,
    Famicom,
    AtomicPurple,
    GameCube,
    Cyberpunk,
}

impl ThemeId {
    pub const ALL: [ThemeId; 10] = [
        ThemeId::Standard,
        ThemeId::GameBoy,
        ThemeId::Nes,
        ThemeId::Commodore64,
        ThemeId::ArcadeCrt,
        ThemeId::VirtualBoy,
        ThemeId::Famicom,
        ThemeId::AtomicPurple,
        ThemeId::GameCube,
        ThemeId::Cyberpunk,
    ];
}

#[derive(Clone, Copy, PartialEq)]
enum Font {
    Default,
    Pixel,
    Terminal,
    Rubik,
    Orbitron,
    TechMono,
}

impl Font {
    fn family(self) -> FontFamily {
        match self {
            Font::Default => FontFamily::Proportional,
            Font::Pixel => FontFamily::Name("pixel".into()),
            Font::Terminal => FontFamily::Name("terminal".into()),
            Font::Rubik => FontFamily::Name("rubik".into()),
            Font::Orbitron => FontFamily::Name("orbitron".into()),
            Font::TechMono => FontFamily::Name("techmono".into()),
        }
    }
    /// How much bigger than a normal UI font this face must be to read the same.
    fn body_scale(self) -> f32 {
        match self {
            Font::Terminal => 1.3,
            _ => 1.0,
        }
    }
    fn head_scale(self) -> f32 {
        match self {
            Font::Pixel => 0.62,
            Font::Orbitron => 0.9,
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Fx {
    None,
    Dither,
    Scanlines,
    Circuit,
    Cyberdeck,
}

#[derive(Clone, Copy)]
struct Palette {
    bg: Color32,
    panel: Color32,
    bar: Color32,
    bar_fg: Color32,
    fg: Color32,
    dim: Color32,
    border: Color32,
    accent: Color32,
    accent2: Color32,
    sel: Color32,
    sel_fg: Color32,
    stripe: Color32,
    input: Color32,
}

pub struct Theme {
    pub id: ThemeId,
    pub name: &'static str,
    pub blurb: &'static str,
    dark: bool,
    p: Palette,
    head: Font,
    body: Font,
    radius: u8,
    stroke: f32,
    fx: Fx,
    /// Red/cyan split behind headings.
    glow: Option<(Color32, Color32)>,
    pub ready: &'static str,
}

const fn c(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
const fn ca(hex: u32, a: u8) -> Color32 {
    // Premultiplied colour with alpha.
    let (r, g, b) = ((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
    Color32::from_rgba_premultiplied(
        ((r as u16 * a as u16) / 255) as u8,
        ((g as u16 * a as u16) / 255) as u8,
        ((b as u16 * a as u16) / 255) as u8,
        a,
    )
}

fn theme(id: ThemeId) -> Theme {
    let base = Theme {
        id,
        name: "Standard",
        blurb: "The usual look; follows your system's light or dark mode.",
        dark: true,
        p: Palette {
            bg: c(0x1b1b1b),
            panel: c(0x222222),
            bar: c(0x2a2a2a),
            bar_fg: c(0xdddddd),
            fg: c(0xdddddd),
            dim: c(0x999999),
            border: c(0x444444),
            accent: c(0x3a7bd5),
            accent2: c(0x3a7bd5),
            sel: c(0x2c5d9b),
            sel_fg: c(0xffffff),
            stripe: c(0x262626),
            input: c(0x101010),
        },
        head: Font::Default,
        body: Font::Default,
        radius: 3,
        stroke: 1.0,
        fx: Fx::None,
        glow: None,
        ready: "Ready",
    };
    match id {
        ThemeId::Standard => base,
        ThemeId::GameBoy => Theme {
            name: "Game Boy",
            blurb: "The original four-shade green screen.",
            dark: false,
            p: Palette {
                bg: c(0x9bbc0f),
                panel: c(0x8bac0f),
                bar: c(0x306230),
                bar_fg: c(0x9bbc0f),
                fg: c(0x0f380f),
                dim: c(0x306230),
                border: c(0x0f380f),
                accent: c(0x0f380f),
                accent2: c(0x306230),
                sel: c(0x306230),
                sel_fg: c(0x9bbc0f),
                stripe: c(0x8bac0f),
                input: c(0x8bac0f),
            },
            head: Font::Pixel,
            body: Font::Terminal,
            radius: 0,
            stroke: 2.0,
            fx: Fx::Dither,
            ..base
        },
        ThemeId::Nes => Theme {
            name: "NES",
            blurb: "Console black, case grey and logo red.",
            p: Palette {
                bg: c(0x1b1b1f),
                panel: c(0x26262b),
                bar: c(0x8e8e94),
                bar_fg: c(0x111114),
                fg: c(0xececec),
                dim: c(0x9a9aa2),
                border: c(0x55555d),
                accent: c(0xe4000f),
                accent2: c(0xe4000f),
                sel: c(0xe4000f),
                sel_fg: c(0xffffff),
                stripe: c(0x222227),
                input: c(0x121215),
            },
            head: Font::Pixel,
            body: Font::Terminal,
            radius: 0,
            stroke: 2.0,
            ..base
        },
        ThemeId::Commodore64 => Theme {
            name: "Commodore 64",
            blurb: "Blue on blue, like the boot screen.",
            p: Palette {
                bg: c(0x352879),
                panel: c(0x3a2d82),
                bar: c(0x6c5eb5),
                bar_fg: c(0x352879),
                fg: c(0xa99df0),
                dim: c(0x7d70c8),
                border: c(0x6c5eb5),
                accent: c(0xbfce72),
                accent2: c(0xbfce72),
                sel: c(0xa99df0),
                sel_fg: c(0x352879),
                stripe: c(0x3a2d82),
                input: c(0x2a1f66),
            },
            head: Font::Pixel,
            body: Font::Terminal,
            radius: 0,
            stroke: 2.0,
            ready: "READY.",
            ..base
        },
        ThemeId::ArcadeCrt => Theme {
            name: "Arcade CRT",
            blurb: "Neon magenta and cyan on black, with scanlines.",
            p: Palette {
                bg: c(0x07060d),
                panel: c(0x0f0d1c),
                bar: c(0x14112a),
                bar_fg: c(0x29e7ff),
                fg: c(0xe9e6ff),
                dim: c(0x8a84b5),
                border: c(0x3a2f6e),
                accent: c(0xff2e88),
                accent2: c(0x29e7ff),
                sel: c(0xff2e88),
                sel_fg: c(0x07060d),
                stripe: c(0x0d0b18),
                input: c(0x050409),
            },
            head: Font::Pixel,
            body: Font::Terminal,
            radius: 0,
            stroke: 2.0,
            fx: Fx::Scanlines,
            glow: Some((ca(0xff2e88, 150), ca(0x29e7ff, 110))),
            ready: "INSERT COIN · Ready",
            ..base
        },
        ThemeId::VirtualBoy => Theme {
            name: "Virtual Boy",
            blurb: "Red on black. Striking, but hard on the eyes.",
            p: Palette {
                bg: c(0x000000),
                panel: c(0x0b0000),
                bar: c(0x3a0000),
                bar_fg: c(0xff1a1a),
                fg: c(0xff1a1a),
                dim: c(0xa30000),
                border: c(0x5c0000),
                accent: c(0xff1a1a),
                accent2: c(0xa30000),
                sel: c(0xff1a1a),
                sel_fg: c(0x000000),
                stripe: c(0x110000),
                input: c(0x0b0000),
            },
            head: Font::Pixel,
            body: Font::Terminal,
            radius: 0,
            stroke: 2.0,
            ..base
        },
        ThemeId::Famicom => Theme {
            name: "Famicom",
            blurb: "Cream, maroon and gold. The easiest to read.",
            dark: false,
            p: Palette {
                bg: c(0xece3cf),
                panel: c(0xf6f0e2),
                bar: c(0x8c1d24),
                bar_fg: c(0xf6f0e2),
                fg: c(0x2a1b17),
                dim: c(0x7b6a5c),
                border: c(0xc9b99a),
                accent: c(0x8c1d24),
                accent2: c(0xa5782a),
                sel: c(0x8c1d24),
                sel_fg: c(0xf6f0e2),
                stripe: c(0xe6dcc5),
                input: c(0xfffbf2),
            },
            head: Font::Pixel,
            body: Font::Default,
            radius: 4,
            stroke: 1.0,
            ..base
        },
        ThemeId::AtomicPurple => Theme {
            name: "Atomic Purple",
            blurb: "See-through purple, with the circuit board showing.",
            p: Palette {
                bg: ca(0x5a3c96, 180),
                panel: ca(0x3a1e68, 150),
                bar: ca(0x281250, 225),
                bar_fg: c(0xd9c6ff),
                fg: c(0xf1e9ff),
                dim: c(0xb9a3e6),
                border: ca(0xc4a8ff, 150),
                accent: c(0xc9a6ff),
                accent2: c(0xffd6f5),
                sel: c(0xd6beff),
                sel_fg: c(0x2a1150),
                stripe: ca(0xffffff, 12),
                input: ca(0x1c0c38, 200),
            },
            head: Font::Pixel,
            body: Font::Terminal,
            radius: 4,
            stroke: 1.0,
            fx: Fx::Circuit,
            ..base
        },
        ThemeId::GameCube => Theme {
            name: "GameCube",
            blurb: "Indigo, with the controller's button colours.",
            p: Palette {
                bg: c(0x1c1a3d),
                panel: c(0x26235a),
                bar: c(0x4d48a8),
                bar_fg: c(0xffffff),
                fg: c(0xeceafc),
                dim: c(0xa9a5e0),
                border: c(0x3b378a),
                accent: c(0x2fb35a),
                accent2: c(0xf6c933),
                sel: c(0x6b66d9),
                sel_fg: c(0xffffff),
                stripe: c(0x211f4a),
                input: c(0x15132f),
            },
            head: Font::Rubik,
            body: Font::Rubik,
            radius: 10,
            stroke: 1.0,
            ..base
        },
        ThemeId::Cyberpunk => Theme {
            name: "Cyberpunk",
            blurb: "A cyberdeck console: HUD frames and an LCD screen.",
            p: Palette {
                bg: ca(0x0a0614, 150),
                panel: ca(0x0a0614, 170),
                bar: ca(0x0d0818, 200),
                bar_fg: c(0x7fd8de),
                fg: c(0xe8f7f8),
                dim: c(0x7fd8de),
                border: c(0xf3e600),
                accent: c(0xf3e600),
                accent2: c(0xff2e88),
                sel: c(0x00d8e6),
                sel_fg: c(0x05030a),
                stripe: ca(0xffffff, 8),
                input: ca(0x05030a, 220),
            },
            head: Font::Orbitron,
            body: Font::TechMono,
            radius: 0,
            stroke: 1.0,
            fx: Fx::Cyberdeck,
            glow: Some((ca(0xff003c, 200), ca(0x00f0ff, 200))),
            ready: "SYS ONLINE · Ready",
            ..base
        },
    }
}

pub fn info(id: ThemeId) -> Theme {
    theme(id)
}

/// Swatches for the theme picker.
pub fn swatches(id: ThemeId) -> [Color32; 4] {
    let t = theme(id);
    [t.p.bar.to_opaque(), t.p.bg.to_opaque(), t.p.accent, t.p.fg]
}

// ------------------------------------------------------------------ fonts

pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (key, bytes) in [
        (
            "pixel",
            &include_bytes!("../../../assets/fonts/PressStart2P-Regular.ttf")[..],
        ),
        (
            "terminal",
            &include_bytes!("../../../assets/fonts/VT323-Regular.ttf")[..],
        ),
        (
            "rubik",
            &include_bytes!("../../../assets/fonts/Rubik.ttf")[..],
        ),
        (
            "orbitron",
            &include_bytes!("../../../assets/fonts/Orbitron.ttf")[..],
        ),
        (
            "techmono",
            &include_bytes!("../../../assets/fonts/ShareTechMono-Regular.ttf")[..],
        ),
    ] {
        fonts
            .font_data
            .insert(key.into(), Arc::new(FontData::from_static(bytes)));
        let mut list = vec![key.to_string()];
        list.extend(fallbacks.iter().cloned());
        fonts.families.insert(FontFamily::Name(key.into()), list);
    }
    ctx.set_fonts(fonts);
}

// ------------------------------------------------------------------ applying

struct Active {
    id: ThemeId,
    head: FontFamily,
    head_scale: f32,
    body_scale: f32,
    glow: Option<(Color32, Color32)>,
    ready: &'static str,
}

static ACTIVE: RwLock<Option<Active>> = RwLock::new(None);

/// Body-text size for the current theme (terminal fonts need a bigger size).
pub fn fs(size: f32) -> f32 {
    ACTIVE
        .read()
        .ok()
        .and_then(|a| a.as_ref().map(|a| a.body_scale))
        .unwrap_or(1.0)
        * size
}

/// Heading text in the current theme's display font.
pub fn head(text: impl Into<String>, size: f32) -> RichText {
    let (fam, scale) = ACTIVE
        .read()
        .ok()
        .and_then(|a| a.as_ref().map(|a| (a.head.clone(), a.head_scale)))
        .unwrap_or((FontFamily::Proportional, 1.0));
    RichText::new(text).family(fam).size(size * scale)
}

pub fn glow() -> Option<(Color32, Color32)> {
    ACTIVE
        .read()
        .ok()
        .and_then(|a| a.as_ref().and_then(|a| a.glow))
}

pub fn ready_text() -> &'static str {
    ACTIVE
        .read()
        .ok()
        .and_then(|a| a.as_ref().map(|a| a.ready))
        .unwrap_or("Ready")
}

pub fn current() -> ThemeId {
    ACTIVE
        .read()
        .ok()
        .and_then(|a| a.as_ref().map(|a| a.id))
        .unwrap_or_default()
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

/// Apply a theme. `base` is the app's normal (Standard) dark and light styles.
pub fn apply(ctx: &Context, id: ThemeId, base: &(egui::Style, egui::Style)) {
    let t = theme(id);
    *ACTIVE.write().unwrap() = Some(Active {
        id,
        head: t.head.family(),
        head_scale: t.head.head_scale(),
        body_scale: t.body.body_scale(),
        glow: t.glow,
        ready: t.ready,
    });
    if id == ThemeId::Standard {
        ctx.set_style_of(egui::Theme::Dark, base.0.clone());
        ctx.set_style_of(egui::Theme::Light, base.1.clone());
        ctx.set_theme(egui::ThemePreference::System);
        return;
    }
    let mut style = if t.dark {
        base.0.clone()
    } else {
        base.1.clone()
    };
    let p = t.p;
    let r = CornerRadius::same(t.radius);
    let v = &mut style.visuals;
    v.dark_mode = t.dark;
    v.override_text_color = None;
    v.weak_text_color = Some(p.dim);
    v.hyperlink_color = p.accent2;
    v.faint_bg_color = p.stripe;
    v.extreme_bg_color = p.input;
    v.text_edit_bg_color = Some(p.input);
    v.code_bg_color = p.panel;
    v.panel_fill = p.bg;
    v.window_fill = if p.panel.a() < 255 {
        mix(Color32::BLACK, p.panel, 0.9).to_opaque()
    } else {
        p.panel
    };
    v.window_stroke = Stroke::new(t.stroke, p.border);
    v.window_corner_radius = r;
    v.menu_corner_radius = r;
    v.selection.bg_fill = p.sel;
    v.selection.stroke = Stroke::new(1.0, p.sel_fg);
    v.warn_fg_color = p.accent2;
    let button = mix(p.panel, p.fg, 0.08);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.panel;
    w.noninteractive.weak_bg_fill = p.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.fg);
    w.noninteractive.corner_radius = r;
    for (ws, fill, stroke, fg) in [
        (&mut w.inactive, button, p.border, p.fg),
        (&mut w.hovered, mix(button, p.accent, 0.22), p.accent, p.fg),
        (&mut w.open, mix(button, p.accent, 0.22), p.accent, p.fg),
        (&mut w.active, mix(button, p.accent, 0.45), p.accent, p.fg),
    ] {
        ws.bg_fill = fill;
        ws.weak_bg_fill = fill;
        ws.bg_stroke = Stroke::new(t.stroke.min(1.5), stroke);
        ws.fg_stroke = Stroke::new(1.5, fg);
        ws.corner_radius = r;
    }
    // Fonts and sizes.
    let body = t.body.family();
    let bs = t.body.body_scale();
    let mono = if t.body == Font::Default {
        FontFamily::Monospace
    } else {
        body.clone()
    };
    style
        .text_styles
        .insert(TextStyle::Small, FontId::new(10.0 * bs, body.clone()));
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.0 * bs, body.clone()));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::new(13.0 * bs, body.clone()));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::new(12.5 * bs, mono));
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(18.0 * t.head.head_scale(), t.head.family()),
    );
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_style_of(egui::Theme::Light, style);
    ctx.set_theme(if t.dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
}

// ------------------------------------------------------------------ painted effects

#[derive(Default)]
pub struct Textures {
    circuit: Option<TextureHandle>,
    cyberdeck: Option<TextureHandle>,
    dither_gb: Option<TextureHandle>,
    dither_gb_dark: Option<TextureHandle>,
    scanlines: Option<TextureHandle>,
    lcd: Option<TextureHandle>,
}

fn load_png(ctx: &Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let img = image::load_from_memory(bytes)
        .expect("bundled image")
        .into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    ctx.load_texture(name, ci, repeat())
}

fn repeat() -> TextureOptions {
    TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Nearest,
        wrap_mode: egui::TextureWrapMode::Repeat,
        mipmap_mode: None,
    }
}

fn pattern(
    ctx: &Context,
    name: &str,
    w: usize,
    h: usize,
    px: impl Fn(usize, usize) -> Color32,
) -> TextureHandle {
    let mut ci = ColorImage::filled([w, h], Color32::TRANSPARENT);
    for y in 0..h {
        for x in 0..w {
            ci[(x, y)] = px(x, y);
        }
    }
    ctx.load_texture(name, ci, repeat())
}

impl Textures {
    fn get(&mut self, ctx: &Context) -> &mut Self {
        if self.circuit.is_none() {
            self.circuit = Some(load_png(
                ctx,
                "circuit",
                include_bytes!("../../../assets/themes/circuit.png"),
            ));
            self.cyberdeck = Some(load_png(
                ctx,
                "cyberdeck",
                include_bytes!("../../../assets/themes/cyberdeck.png"),
            ));
            self.dither_gb = Some(pattern(ctx, "dither-light", 2, 2, |x, y| {
                if (x + y) % 2 == 0 {
                    c(0x8bac0f)
                } else {
                    c(0x306230)
                }
            }));
            self.dither_gb_dark = Some(pattern(ctx, "dither-dark", 2, 2, |x, y| {
                if (x + y) % 2 == 0 {
                    c(0x306230)
                } else {
                    c(0x0f380f)
                }
            }));
            self.scanlines = Some(pattern(ctx, "scanlines", 1, 3, |_, y| {
                if y == 2 {
                    ca(0x000000, 75)
                } else {
                    Color32::TRANSPARENT
                }
            }));
            self.lcd = Some(pattern(ctx, "lcd", 3, 3, |x, y| {
                if y == 2 {
                    ca(0x000000, 70)
                } else {
                    [ca(0xff2828, 18), ca(0x28ff5a, 15), ca(0x3c5aff, 20)][x]
                }
            }));
        }
        self
    }
}

/// A texture repeated over `rect` at its natural size (in points).
fn tile(painter: &Painter, tex: &TextureHandle, rect: Rect, tint: Color32) {
    let [w, h] = tex.size();
    let uv = Rect::from_min_max(
        Pos2::ZERO,
        Pos2::new(rect.width() / w as f32, rect.height() / h as f32),
    );
    painter.image(tex.id(), rect, uv, tint);
}

/// A vertical gradient rectangle.
fn vgrad(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(rect.left_top(), top);
    m.colored_vertex(rect.right_top(), top);
    m.colored_vertex(rect.left_bottom(), bottom);
    m.colored_vertex(rect.right_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 3, 2);
    painter.add(Shape::mesh(m));
}

/// Darkened corners, like a curved screen.
fn vignette(painter: &Painter, rect: Rect, strength: u8) {
    let edge = Color32::from_black_alpha(strength);
    let mut m = Mesh::default();
    let inner = rect.shrink2(rect.size() * 0.22);
    let pts = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ];
    let ins = [
        inner.left_top(),
        inner.right_top(),
        inner.right_bottom(),
        inner.left_bottom(),
    ];
    for i in 0..4 {
        m.vertices.push(Vertex {
            pos: pts[i],
            uv: WHITE_UV,
            color: edge,
        });
        m.vertices.push(Vertex {
            pos: ins[i],
            uv: WHITE_UV,
            color: Color32::TRANSPARENT,
        });
    }
    for i in 0..4u32 {
        let (o0, i0, o1, i1) = (2 * i, 2 * i + 1, (2 * i + 2) % 8, (2 * i + 3) % 8);
        m.add_triangle(o0, o1, i0);
        m.add_triangle(o1, i1, i0);
    }
    painter.add(Shape::mesh(m));
}

/// Paint the window background (under every panel). Call at the start of each frame.
pub fn paint_background(ctx: &Context, tex: &mut Textures) {
    let t = theme(current());
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(LayerId::background());
    match t.fx {
        Fx::Circuit => {
            let tx = tex.get(ctx);
            tile(&painter, tx.circuit.as_ref().unwrap(), rect, Color32::WHITE);
        }
        Fx::Cyberdeck => {
            let tx = tex.get(ctx);
            vgrad(&painter, rect, c(0x140b28), c(0x05030a));
            let glow = Rect::from_center_size(rect.center(), rect.size() * 0.8);
            painter.rect_filled(glow, CornerRadius::same(255), ca(0x5c1e96, 40));
            tile(
                &painter,
                tx.cyberdeck.as_ref().unwrap(),
                rect,
                Color32::WHITE,
            );
        }
        _ => {}
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Area {
    /// The navigation sidebar.
    Side,
    /// The main content area.
    Main,
    /// The status bar.
    Bar,
}

/// Shading (and for some themes, frames) for a panel. Call first thing inside the panel.
pub fn paint_panel(ui: &mut Ui, area: Area, tex: &mut Textures) {
    let t = theme(current());
    if t.id == ThemeId::Standard {
        return;
    }
    if area == Area::Bar {
        // Text on the status bar uses the bar's own text colour.
        ui.visuals_mut().override_text_color = Some(t.p.bar_fg);
        ui.visuals_mut().weak_text_color = Some(t.p.bar_fg);
    }
    let rect = ui.max_rect();
    let painter = ui.painter();
    let p = t.p;
    match (t.fx, area) {
        (Fx::Dither, Area::Bar) => {
            painter.rect_filled(rect, 0, p.bar);
            let tx = tex.get(ui.ctx());
            let band = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - 6.0), rect.max);
            tile(
                painter,
                tx.dither_gb_dark.as_ref().unwrap(),
                band,
                Color32::WHITE,
            );
        }
        (Fx::Dither, Area::Side) => {
            let tx = tex.get(ui.ctx());
            let band = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - 10.0), rect.max);
            tile(
                painter,
                tx.dither_gb.as_ref().unwrap(),
                band,
                Color32::WHITE,
            );
        }
        (Fx::Dither, Area::Main) => {}
        (_, Area::Bar) => {
            vgrad(
                painter,
                rect,
                mix(p.bar, Color32::WHITE, 0.14),
                mix(p.bar, Color32::BLACK, 0.16),
            );
            painter.hline(
                rect.x_range(),
                rect.top() + 0.5,
                Stroke::new(1.0, Color32::from_white_alpha(30)),
            );
        }
        (Fx::Cyberdeck, _) => {}
        (_, Area::Side) => {
            vgrad(
                painter,
                rect,
                mix(p.panel, Color32::WHITE, 0.05),
                mix(p.panel, Color32::BLACK, 0.14),
            );
        }
        (_, Area::Main) => {
            vgrad(
                painter,
                rect,
                mix(p.bg, Color32::WHITE, 0.04),
                mix(p.bg, Color32::BLACK, 0.12),
            );
        }
    }
    if t.fx == Fx::Cyberdeck {
        let (col, inset) = match area {
            Area::Side => (c(0x00f0ff), 6.0),
            Area::Main => (c(0xf3e600), 4.0),
            Area::Bar => (c(0xff2e88), 2.0),
        };
        hud_frame(painter, rect.shrink(inset), col, area != Area::Bar);
        if area == Area::Bar {
            let y = rect.center().y;
            for (i, col) in [c(0xff2e88), c(0xff2e88), c(0xf3e600), c(0x00f0ff)]
                .iter()
                .enumerate()
            {
                let x = rect.right() - 70.0 + i as f32 * 12.0;
                painter.rect_filled(
                    Rect::from_center_size(Pos2::new(x, y), Vec2::splat(7.0)),
                    0,
                    *col,
                );
            }
        }
    }
}

/// An outlined HUD frame with cut corners, notches and tick marks.
fn hud_frame(painter: &Painter, r: Rect, col: Color32, ticks: bool) {
    let k = 12.0_f32.min(r.height() / 3.0);
    let pts = vec![
        Pos2::new(r.left() + k, r.top()),
        Pos2::new(r.right() - k, r.top()),
        Pos2::new(r.right(), r.top() + k),
        Pos2::new(r.right(), r.bottom() - k),
        Pos2::new(r.right() - k, r.bottom()),
        Pos2::new(r.left() + k, r.bottom()),
        Pos2::new(r.left(), r.bottom() - k),
        Pos2::new(r.left(), r.top() + k),
    ];
    painter.add(Shape::closed_line(pts, Stroke::new(1.5, col)));
    if ticks {
        let fill = |x: f32, y: f32, w: f32, h: f32| {
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h)),
                0,
                col,
            )
        };
        fill(r.right() - k - 22.0, r.top() + 3.0, 8.0, 2.0);
        fill(r.right() - k - 34.0, r.top() + 3.0, 5.0, 2.0);
        fill(r.left() + 2.0, r.top() + k + 4.0, 2.0, 12.0);
        fill(r.right() - 4.0, r.bottom() - k - 16.0, 2.0, 12.0);
        fill(r.left() + k + 6.0, r.bottom() - 4.0, 16.0, 2.0);
    }
}

/// The screen effects a theme comes with: (CRT scanlines, LCD screen).
pub fn default_effects(id: ThemeId) -> (bool, bool) {
    match theme(id).fx {
        Fx::Scanlines => (true, false),
        Fx::Cyberdeck => (false, true),
        _ => (false, false),
    }
}

/// Screen-effect overlays drawn on top of everything. Call at the end of each frame.
pub fn paint_overlay(ctx: &Context, tex: &mut Textures, crt: bool, lcd: bool) {
    if !crt && !lcd {
        return;
    }
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(LayerId::new(
        egui::Order::Tooltip,
        egui::Id::unique("theme-overlay"),
    ));
    let tx = tex.get(ctx);
    if lcd {
        tile(&painter, tx.lcd.as_ref().unwrap(), rect, Color32::WHITE);
        // Glass glare across the top-left.
        let glare = Rect::from_min_size(rect.min, rect.size() * Vec2::new(0.45, 0.35));
        let mut m = Mesh::default();
        m.colored_vertex(glare.left_top(), Color32::from_white_alpha(14));
        m.colored_vertex(glare.right_top(), Color32::TRANSPARENT);
        m.colored_vertex(glare.left_bottom(), Color32::TRANSPARENT);
        m.add_triangle(0, 1, 2);
        painter.add(Shape::mesh(m));
    }
    if crt {
        tile(
            &painter,
            tx.scanlines.as_ref().unwrap(),
            rect,
            Color32::WHITE,
        );
    }
    vignette(&painter, rect, if lcd { 120 } else { 90 });
}
