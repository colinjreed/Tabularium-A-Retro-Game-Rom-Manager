//! The finishing touches for each theme: pixel-art icons, shaped menu buttons,
//! line art in the sidebar, start-up screens and small animations.

use super::{c, ca, current, mix, theme, vgrad, Fx, Theme, ThemeId};
use egui::{
    Align2, Color32, Context, CornerRadius, FontId, LayerId, Painter, Pos2, Rect, Response, Sense,
    Shape, Stroke, TextStyle, Ui, Vec2,
};

// ------------------------------------------------------------------ icons

#[derive(Clone, Copy)]
pub enum Icon {
    Logo,
    Library,
    Import,
    Clean,
    Artwork,
    Bios,
    Sd,
    Themes,
    Settings,
}

/// 12x12 pixel-art icons.
fn bitmap(icon: Icon) -> [&'static str; 12] {
    match icon {
        Icon::Logo => [
            "............",
            "............",
            "..########..",
            ".#........#.",
            "#..#.......#",
            "#.###...#..#",
            "#..#...#.#.#",
            "#.......#..#",
            "#..........#",
            ".#..####..#.",
            "..##....##..",
            "............",
        ],
        Icon::Library => [
            "............",
            "###.###.###.",
            "#.#.#.#.#.#.",
            "###.###.###.",
            "#.#.#.#.#.#.",
            "#.#.#.#.#.#.",
            "#.#.#.#.#.#.",
            "#.#.#.#.#.#.",
            "###.###.###.",
            "#.#.#.#.#.#.",
            "############",
            "............",
        ],
        Icon::Import => [
            ".....##.....",
            ".....##.....",
            ".....##.....",
            ".....##.....",
            "..##.##.##..",
            "...######...",
            "....####....",
            ".....##.....",
            "##........##",
            "##........##",
            "############",
            "............",
        ],
        Icon::Clean => [
            "....####....",
            "############",
            "............",
            ".##########.",
            ".#..#..#..#.",
            ".#..#..#..#.",
            ".#..#..#..#.",
            ".#..#..#..#.",
            ".#..#..#..#.",
            ".#..#..#..#.",
            "..########..",
            "............",
        ],
        Icon::Artwork => [
            "............",
            "############",
            "#..........#",
            "#.......##.#",
            "#.......##.#",
            "#...#......#",
            "#..###.....#",
            "#.#####.#..#",
            "#######.##.#",
            "############",
            "............",
            "............",
        ],
        Icon::Bios => [
            "............",
            "..#.#..#.#..",
            ".##########.",
            "##........##",
            ".#.######.#.",
            "##.#....#.##",
            ".#.#....#.#.",
            "##.######.##",
            ".#........#.",
            ".##########.",
            "..#.#..#.#..",
            "............",
        ],
        Icon::Sd => [
            "............",
            "...#######..",
            "..#.#.#.#.#.",
            ".##.#.#.#.#.",
            ".#........#.",
            ".#........#.",
            ".#........#.",
            ".#........#.",
            ".#........#.",
            ".#........#.",
            ".##########.",
            "............",
        ],
        Icon::Themes => [
            "....#####...",
            "..##.....##.",
            ".#..##.....#",
            "#...##.##..#",
            "#.......##.#",
            "#.##.......#",
            "#.##....##.#",
            "#.......#..#",
            ".#.....#..#.",
            "..#....#.#..",
            "...####.##..",
            "............",
        ],
        Icon::Settings => [
            ".....##.....",
            ".#..####..#.",
            "..########..",
            ".###....###.",
            ".##......##.",
            "###......###",
            "###......###",
            ".##......##.",
            ".###....###.",
            "..########..",
            ".#..####..#.",
            ".....##.....",
        ],
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Dots {
    Square,
    Round,
    Led,
}

fn dots(id: ThemeId) -> Option<Dots> {
    match id {
        ThemeId::Standard => None,
        ThemeId::GameCube => Some(Dots::Round),
        ThemeId::Cyberpunk => Some(Dots::Led),
        _ => Some(Dots::Square),
    }
}

/// Whether the current theme draws its own pixel-art icons.
pub fn has_pixel_icons() -> bool {
    dots(current()).is_some()
}

/// Paint a pixel-art icon filling `rect` (best at 24 or 36 points square).
pub fn paint_icon(painter: &Painter, rect: Rect, icon: Icon, col: Color32) {
    let style = dots(current()).unwrap_or(Dots::Square);
    let px = (rect.width() / 12.0).floor().max(1.0);
    let origin = rect.min.round();
    for (y, row) in bitmap(icon).iter().enumerate() {
        for (x, ch) in row.bytes().enumerate() {
            if ch != b'#' {
                continue;
            }
            let cell = Rect::from_min_size(
                origin + Vec2::new(x as f32 * px, y as f32 * px),
                Vec2::splat(px),
            );
            match style {
                Dots::Square => {
                    painter.rect_filled(cell, 0, col);
                }
                Dots::Round => {
                    painter.circle_filled(cell.center(), px * 0.55, col);
                }
                Dots::Led => {
                    painter.rect_filled(cell.expand(px * 0.6), 1, col.gamma_multiply(0.18));
                    painter.rect_filled(cell.shrink(px * 0.1), 0, col);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ menu buttons

#[derive(Clone, Copy, PartialEq)]
enum ButtonShape {
    Plain,
    /// Raised, like a cartridge label or a GBC button.
    Bevel,
    /// Rounded ends, like START/SELECT or the GameCube's buttons.
    Pill,
    /// Square reverse video, like a C64 cursor.
    Inverse,
    /// Neon tube outline.
    Neon,
    /// Cut corners and a glow.
    Chamfer,
}

fn button_shape(id: ThemeId) -> ButtonShape {
    match id {
        ThemeId::Standard => ButtonShape::Plain,
        ThemeId::Nes | ThemeId::Famicom | ThemeId::AtomicPurple => ButtonShape::Bevel,
        ThemeId::GameBoy | ThemeId::GameCube => ButtonShape::Pill,
        ThemeId::Commodore64 | ThemeId::VirtualBoy => ButtonShape::Inverse,
        ThemeId::ArcadeCrt => ButtonShape::Neon,
        ThemeId::Cyberpunk => ButtonShape::Chamfer,
    }
}

fn chamfer(r: Rect, k: f32) -> Vec<Pos2> {
    vec![
        Pos2::new(r.left() + k, r.top()),
        Pos2::new(r.right(), r.top()),
        Pos2::new(r.right(), r.bottom() - k),
        Pos2::new(r.right() - k, r.bottom()),
        Pos2::new(r.left(), r.bottom()),
        Pos2::new(r.left(), r.top() + k),
    ]
}

fn paint_button(painter: &Painter, rect: Rect, t: &Theme, selected: bool, hovered: bool, ui: &Ui) {
    let p = t.p;
    let shape = button_shape(t.id);
    if shape == ButtonShape::Plain {
        let v = ui.visuals();
        if selected {
            painter.rect_filled(rect, 4, v.selection.bg_fill);
        } else if hovered {
            painter.rect_filled(rect, 4, v.widgets.hovered.weak_bg_fill);
        }
        return;
    }
    let hover_fill = mix(p.panel, p.accent, 0.22).to_opaque();
    let fill = if selected {
        p.sel
    } else if hovered {
        hover_fill
    } else {
        return;
    };
    let shadow = Color32::from_black_alpha(if t.dark { 110 } else { 60 });
    match shape {
        ButtonShape::Plain => {}
        ButtonShape::Bevel => {
            painter.rect_filled(rect.translate(Vec2::new(2.0, 2.0)), 3, shadow);
            painter.rect_filled(rect, 3, fill);
            let hi = Stroke::new(2.0, mix(fill, Color32::WHITE, 0.35));
            let lo = Stroke::new(2.0, mix(fill, Color32::BLACK, 0.35));
            let r = rect.shrink(1.0);
            painter.line_segment([r.left_bottom(), r.left_top()], hi);
            painter.line_segment([r.left_top(), r.right_top()], hi);
            painter.line_segment([r.right_top(), r.right_bottom()], lo);
            painter.line_segment([r.right_bottom(), r.left_bottom()], lo);
        }
        ButtonShape::Pill => {
            let round = CornerRadius::same((rect.height() / 2.0) as u8);
            painter.rect_filled(rect.translate(Vec2::new(0.0, 2.0)), round, shadow);
            painter.rect_filled(rect, round, fill);
            // A soft highlight on the upper half.
            let top = Rect::from_min_max(
                rect.min + Vec2::new(rect.height() / 2.0, 3.0),
                Pos2::new(rect.right() - rect.height() / 2.0, rect.center().y - 2.0),
            );
            painter.rect_filled(top, 4, Color32::from_white_alpha(22));
        }
        ButtonShape::Inverse => {
            painter.rect_filled(rect, 0, fill);
        }
        ButtonShape::Neon => {
            let glow = if selected { p.sel } else { p.accent };
            painter.rect_filled(rect, 3, fill);
            for (w, a) in [(7.0, 0.10), (4.0, 0.22)] {
                painter.rect_stroke(
                    rect,
                    3,
                    Stroke::new(w, glow.gamma_multiply(a)),
                    egui::StrokeKind::Outside,
                );
            }
            painter.rect_stroke(
                rect,
                3,
                Stroke::new(1.5, mix(glow, Color32::WHITE, 0.4)),
                egui::StrokeKind::Inside,
            );
        }
        ButtonShape::Chamfer => {
            let k = 8.0;
            painter.add(Shape::convex_polygon(
                chamfer(rect.translate(Vec2::new(3.0, 3.0)), k),
                Color32::from_black_alpha(140),
                Stroke::NONE,
            ));
            painter.add(Shape::convex_polygon(
                chamfer(rect.expand(2.0), k + 1.0),
                fill.gamma_multiply(0.25),
                Stroke::NONE,
            ));
            painter.add(Shape::convex_polygon(
                chamfer(rect, k),
                fill,
                Stroke::new(1.0, mix(fill, Color32::WHITE, 0.45)),
            ));
        }
    }
}

/// A sidebar menu entry in the current theme's style.
pub fn nav_item(ui: &mut Ui, selected: bool, icon: Icon, emoji: &str, text: &str) -> Response {
    let t = theme(current());
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::click());
    let rect = rect.shrink2(Vec2::new(2.0, 1.0));
    let painter = ui.painter();
    paint_button(painter, rect, &t, selected, resp.hovered(), ui);
    let fg = if t.id == ThemeId::Standard {
        if selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        }
    } else if selected {
        t.p.sel_fg
    } else {
        t.p.fg
    };
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 10.0, rect.center().y - 12.0),
        Vec2::splat(24.0),
    );
    if dots(t.id).is_some() {
        paint_icon(painter, icon_rect, icon, fg);
    } else {
        painter.text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            emoji,
            FontId::proportional(17.0),
            fg,
        );
    }
    let family = TextStyle::Body.resolve(ui.style()).family;
    painter.text(
        Pos2::new(rect.left() + 44.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::new(super::fs(15.0), family),
        fg,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ------------------------------------------------------------------ sidebar art

/// Faint line art of the theme's hardware, for the empty bottom of the sidebar.
pub fn paint_side_art(painter: &Painter, area: Rect) {
    let t = theme(current());
    let s = (area.width() - 36.0).min(area.height() - 16.0).min(150.0);
    if s < 70.0 || t.id == ThemeId::Standard {
        return;
    }
    let col = t.p.fg.gamma_multiply(if t.dark { 0.16 } else { 0.22 });
    let st = Stroke::new(2.0, col);
    let b = Rect::from_center_size(
        Pos2::new(area.center().x, area.bottom() - 8.0 - s / 2.0),
        Vec2::splat(s),
    );
    let at = |x: f32, y: f32| Pos2::new(b.left() + x * s, b.top() + y * s);
    let dpad = |c: Pos2, arm: f32| {
        let th = arm * 0.7;
        painter.rect_filled(Rect::from_center_size(c, Vec2::new(arm * 2.6, th)), 1, col);
        painter.rect_filled(Rect::from_center_size(c, Vec2::new(th, arm * 2.6)), 1, col);
    };
    match t.id {
        ThemeId::AtomicPurple => {
            // A speaker grille moulded into the case: staggered holes in a
            // round patch, each dark inside with a lit lower lip.
            // Everything on the board is seen through the case, like the board
            // background: draw it at the same strength so it matches.
            let board = {
                let mut p = painter.clone();
                p.multiply_opacity(super::see_through());
                p
            };
            let ctr = b.center();
            let radius = s * 0.46;
            let pitch = (s / 11.0).max(7.0);
            let hole = pitch * 0.3;
            // The speaker on the board, seen through the plastic: metal frame,
            // ribbed cone, centre cap, and its wires to the board.
            // Solid, like the board image; `board` makes it see-through.
            let tint = |hex: u32, _a: u8| c(hex);
            // Copper traces, pads, an amplifier chip and its capacitors,
            // drawn first so the speaker sits on top of the board.
            let copper = tint(0x2f8a63, 230);
            let trace = |pts: Vec<Pos2>, w: f32| {
                board.add(Shape::line(pts, Stroke::new(w, copper)));
            };
            let at = |x: f32, y: f32| ctr + Vec2::new(radius * x, radius * y);
            let pad_p = at(1.02, -0.98);
            let pad_n = at(0.78, -1.18);
            let amp = Rect::from_center_size(at(0.05, -1.62), Vec2::splat(radius * 0.32));
            // Pads to the amplifier, with 45-degree bends.
            trace(
                vec![
                    pad_p,
                    at(1.02, -1.3),
                    at(0.72, -1.6),
                    amp.right_center() + Vec2::new(0.0, 3.0),
                ],
                2.2,
            );
            trace(
                vec![
                    pad_n,
                    at(0.6, -1.36),
                    amp.right_center() - Vec2::new(0.0, 3.0),
                ],
                2.2,
            );
            // Supply and signal traces leaving the amplifier for the rest of the board.
            for (i, y) in [-0.1f32, 0.0, 0.1].iter().enumerate() {
                let p0 = amp.left_center() + Vec2::new(0.0, radius * y);
                let p1 = p0 - Vec2::new(radius * (0.25 + i as f32 * 0.08), 0.0);
                trace(
                    vec![p0, p1, p1 - Vec2::new(radius * 0.3, radius * 0.3)],
                    1.4,
                );
            }
            trace(
                vec![
                    amp.center_top(),
                    amp.center_top() - Vec2::new(0.0, radius * 0.35),
                ],
                1.4,
            );
            // Vias where traces dive through the board.
            for p in [
                at(-0.62, -1.92),
                at(-0.7, -1.9),
                at(-0.8, -1.88),
                at(0.05, -2.14),
            ] {
                board.circle_filled(p, 2.2, c(0xc8a454));
                board.circle_filled(p, 0.9, tint(0x143a2c, 255));
            }
            // The amplifier chip with its pins.
            for k in 0..4 {
                let f = -0.75 + k as f32 * 0.5;
                for (a, b) in [(amp.left(), -1.0), (amp.right(), 1.0)] {
                    let y = amp.center().y + f * amp.height() * 0.4;
                    board.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(a.min(a + b * 3.0), y - 1.0),
                            Vec2::new(3.0, 2.0),
                        ),
                        0,
                        tint(0xc4c6cc, 230),
                    );
                }
            }
            board.rect_filled(amp, 1, tint(0x1c1c22, 240));
            board.circle_filled(amp.min + Vec2::splat(3.0), 1.2, tint(0x50505a, 255));
            // Decoupling capacitors next to it.
            for (x, y) in [(-0.38, -1.42), (-0.38, -1.62), (0.45, -1.86)] {
                let r = Rect::from_center_size(at(x, y), Vec2::new(7.0, 3.5));
                board.rect_filled(r, 0, tint(0x8a6e50, 235));
                board.rect_filled(r.with_max_x(r.left() + 1.5), 0, tint(0xc8c8c4, 235));
                board.rect_filled(r.with_min_x(r.right() - 1.5), 0, tint(0xc8c8c4, 235));
            }
            // Speaker pads and silkscreen.
            for p in [pad_p, pad_n] {
                board.circle_filled(p, 4.0, c(0xc8a454));
                board.circle_filled(p, 1.6, tint(0x8a6a30, 255));
            }
            let silk = Color32::from_white_alpha(150);
            let font = FontId::monospace(7.0);
            board.text(
                pad_p + Vec2::new(6.0, 0.0),
                Align2::LEFT_CENTER,
                "SPK+",
                font.clone(),
                silk,
            );
            board.text(
                pad_n + Vec2::new(-6.0, -6.0),
                Align2::RIGHT_CENTER,
                "SPK-",
                font.clone(),
                silk,
            );
            board.text(
                amp.center_bottom() + Vec2::new(0.0, 3.0),
                Align2::CENTER_TOP,
                "U12",
                font,
                silk,
            );
            // Speaker wires from its tabs to the pads.
            let wire = |from: Pos2, to: Pos2, col: u32| {
                let mid = from.lerp(to, 0.5) + Vec2::new(4.0, 2.0);
                board.add(Shape::line(
                    vec![from, mid, to],
                    Stroke::new(2.2, tint(col, 210)),
                ));
            };
            let tab_p = at(0.66, -0.56);
            let tab_n = at(0.5, -0.7);
            board.circle_filled(ctr, radius + 6.0, tint(0x2a2a31, 215));
            board.circle_stroke(ctr, radius + 6.0, Stroke::new(1.5, tint(0xa8a8b4, 170)));
            board.circle_stroke(ctr, radius + 1.5, Stroke::new(1.0, tint(0x6a6a74, 150)));
            board.circle_filled(ctr, radius * 0.9, tint(0x141418, 225));
            for k in 1..6 {
                let r = radius * (0.32 + k as f32 * 0.11);
                board.circle_stroke(ctr, r, Stroke::new(1.0, Color32::from_white_alpha(14)));
            }
            board.circle_filled(ctr, radius * 0.3, tint(0x2e2e36, 235));
            board.circle_stroke(
                ctr - Vec2::new(radius * 0.06, radius * 0.06),
                radius * 0.2,
                Stroke::new(1.5, Color32::from_white_alpha(28)),
            );
            for tab in [tab_p, tab_n] {
                board.rect_filled(
                    Rect::from_center_size(tab, Vec2::splat(5.0)),
                    1,
                    c(0xc8a454),
                );
            }
            wire(tab_p, pad_p, 0xc8322e);
            wire(tab_n, pad_n, 0x1a1a1e);
            let rows = (radius / (pitch * 0.866)).ceil() as i32;
            for row in -rows..=rows {
                let y = row as f32 * pitch * 0.866;
                let shift = if row % 2 == 0 { 0.0 } else { pitch * 0.5 };
                let cols = (radius / pitch).ceil() as i32 + 1;
                for col in -cols..=cols {
                    let p = ctr + Vec2::new(col as f32 * pitch + shift, y);
                    if (p - ctr).length() > radius - hole {
                        continue;
                    }
                    painter.circle_filled(p, hole + 1.2, Color32::from_white_alpha(18));
                    painter.circle_filled(p, hole, Color32::from_black_alpha(120));
                    painter.circle_filled(
                        p - Vec2::new(0.0, hole * 0.25),
                        hole * 0.7,
                        Color32::from_black_alpha(70),
                    );
                }
            }
            // The moulded rim around the grille.
            painter.circle_stroke(
                ctr,
                radius + 3.0,
                Stroke::new(1.5, Color32::from_black_alpha(60)),
            );
            painter.circle_stroke(
                ctr,
                radius + 4.5,
                Stroke::new(1.0, Color32::from_white_alpha(30)),
            );
        }
        ThemeId::GameBoy => {
            let w = s * 0.66;
            let body = Rect::from_center_size(b.center(), Vec2::new(w, s));
            let x = |f: f32| body.left() + f * w;
            let y = |f: f32| body.top() + f * s;
            painter.rect_stroke(
                body,
                CornerRadius {
                    nw: 8,
                    ne: 8,
                    sw: 8,
                    se: 30,
                },
                st,
                egui::StrokeKind::Middle,
            );
            let bezel = Rect::from_min_max(Pos2::new(x(0.1), y(0.07)), Pos2::new(x(0.9), y(0.48)));
            painter.rect_stroke(bezel, 4, st, egui::StrokeKind::Middle);
            painter.rect_stroke(bezel.shrink(w * 0.12), 0, st, egui::StrokeKind::Middle);
            dpad(Pos2::new(x(0.26), y(0.66)), w * 0.08);
            painter.circle_filled(Pos2::new(x(0.66), y(0.68)), w * 0.075, col);
            painter.circle_filled(Pos2::new(x(0.84), y(0.62)), w * 0.075, col);
            for f in [0.38, 0.56] {
                painter.rect_filled(
                    Rect::from_center_size(
                        Pos2::new(x(f), y(0.82)),
                        Vec2::new(w * 0.13, s * 0.025),
                    ),
                    3,
                    col,
                );
            }
            for i in 0..4 {
                let o = i as f32 * 0.045;
                painter.line_segment(
                    [
                        Pos2::new(x(0.68 + o), y(0.96)),
                        Pos2::new(x(0.8 + o), y(0.86)),
                    ],
                    st,
                );
            }
        }
        ThemeId::Nes | ThemeId::Famicom => {
            let body = Rect::from_min_max(at(0.0, 0.5), at(1.0, 0.96));
            painter.rect_stroke(body, 4, st, egui::StrokeKind::Middle);
            painter.rect_stroke(body.shrink(6.0), 2, st, egui::StrokeKind::Middle);
            dpad(at(0.19, 0.73), s * 0.055);
            for f in [0.42, 0.56] {
                painter.rect_filled(
                    Rect::from_center_size(at(f, 0.77), Vec2::new(s * 0.1, s * 0.03)),
                    3,
                    col,
                );
            }
            painter.circle_filled(at(0.74, 0.77), s * 0.055, col);
            painter.circle_filled(at(0.88, 0.77), s * 0.055, col);
            // The cable.
            painter.add(Shape::line(
                vec![at(0.5, 0.5), at(0.5, 0.35), at(0.62, 0.2), at(0.6, 0.05)],
                st,
            ));
        }
        ThemeId::Commodore64 => {
            let pts = vec![at(0.0, 0.92), at(1.0, 0.92), at(0.95, 0.55), at(0.05, 0.55)];
            painter.add(Shape::closed_line(pts, st));
            for row in 0..4 {
                let y = 0.6 + row as f32 * 0.075;
                let inset = 0.09 + row as f32 * 0.012;
                for k in 0..10 {
                    let x = inset + k as f32 * ((1.0 - 2.0 * inset) / 10.0);
                    painter.rect_filled(
                        Rect::from_min_size(at(x + 0.008, y), Vec2::new(s * 0.068, s * 0.05)),
                        1,
                        col,
                    );
                }
            }
            for (i, hex) in [0xe04040, 0xe09030, 0xe0d040, 0x40b060, 0x4060e0]
                .iter()
                .enumerate()
            {
                let x = 0.55 + i as f32 * 0.07;
                painter.add(Shape::convex_polygon(
                    vec![
                        at(x, 0.5),
                        at(x + 0.05, 0.5),
                        at(x + 0.17, 0.25),
                        at(x + 0.12, 0.25),
                    ],
                    c(*hex).gamma_multiply(0.35),
                    Stroke::NONE,
                ));
            }
        }
        ThemeId::ArcadeCrt => {
            let base = Rect::from_center_size(at(0.28, 0.9), Vec2::new(s * 0.36, s * 0.1));
            painter.rect_stroke(base, 12, st, egui::StrokeKind::Middle);
            painter.line_segment([at(0.28, 0.88), at(0.24, 0.52)], Stroke::new(4.0, col));
            painter.circle_filled(at(0.235, 0.47), s * 0.09, col);
            for (x, y) in [
                (0.6, 0.72),
                (0.75, 0.68),
                (0.9, 0.66),
                (0.6, 0.88),
                (0.75, 0.84),
                (0.9, 0.82),
            ] {
                painter.circle_stroke(at(x, y), s * 0.055, st);
                painter.circle_filled(at(x, y), s * 0.03, col);
            }
        }
        ThemeId::VirtualBoy => {
            let hz = 0.35;
            let vp = at(0.5, hz);
            for i in 0..=8 {
                let x = -0.4 + i as f32 * 0.225;
                painter.line_segment([vp, at(x, 1.0)], Stroke::new(1.0, col));
            }
            for k in 1..=7 {
                let f = (k as f32 / 7.0).powi(2);
                let y = hz + (1.0 - hz) * f;
                painter.line_segment([at(0.0, y), at(1.0, y)], Stroke::new(1.0, col));
            }
            painter.circle_stroke(at(0.5, 0.2), s * 0.12, st);
        }
        ThemeId::GameCube => {
            let iso = |x: f32, y: f32, z: f32| {
                b.center() + Vec2::new((x - z) * 0.866, (x + z) * 0.5 - y) * (s * 0.3)
            };
            for (k, a) in [(1.0, 1.0), (0.55, 0.6)] {
                let p = |x: f32, y: f32, z: f32| iso(x * k, y * k, z * k);
                let col = col.gamma_multiply(a);
                let st = Stroke::new(2.0, col);
                let top = [
                    p(-1., 1., -1.),
                    p(1., 1., -1.),
                    p(1., 1., 1.),
                    p(-1., 1., 1.),
                ];
                let bot = [
                    p(-1., -1., -1.),
                    p(1., -1., -1.),
                    p(1., -1., 1.),
                    p(-1., -1., 1.),
                ];
                for i in 0..4 {
                    painter.line_segment([top[i], top[(i + 1) % 4]], st);
                    painter.line_segment([bot[i], bot[(i + 1) % 4]], st);
                    if i != 0 {
                        painter.line_segment([top[i], bot[i]], st);
                    }
                }
            }
        }
        ThemeId::Cyberpunk => {
            let ctr = b.center();
            for (r, w) in [(0.48, 1.5), (0.34, 1.0), (0.2, 1.0)] {
                painter.circle_stroke(ctr, s * r, Stroke::new(w, col));
            }
            for i in 0..36 {
                let a = i as f32 * std::f32::consts::TAU / 36.0;
                let d = Vec2::angled(a);
                let inner = if i % 3 == 0 { 0.42 } else { 0.45 };
                painter.line_segment([ctr + d * s * inner, ctr + d * s * 0.48], st);
            }
            painter.line_segment([ctr - Vec2::X * s * 0.5, ctr + Vec2::X * s * 0.5], st);
            painter.line_segment([ctr - Vec2::Y * s * 0.5, ctr + Vec2::Y * s * 0.5], st);
            for (x, y) in [(0.13, -0.2), (-0.25, 0.1), (0.3, 0.22)] {
                painter.circle_filled(
                    ctr + Vec2::new(x, y) * s,
                    3.0,
                    c(0x00f0ff).gamma_multiply(0.5),
                );
            }
        }
        ThemeId::Standard => {}
    }
}

// ------------------------------------------------------------------ animation

fn now(ctx: &Context) -> f64 {
    ctx.input(|i| i.time)
}

/// For themes with a blinking cursor: whether it's lit now.
pub fn cursor_lit(ctx: &Context) -> Option<bool> {
    if !matches!(
        current(),
        ThemeId::Commodore64 | ThemeId::ArcadeCrt | ThemeId::Cyberpunk | ThemeId::VirtualBoy
    ) {
        return None;
    }
    let t = now(ctx);
    let phase = (t * 2.0).fract();
    ctx.request_repaint_after(std::time::Duration::from_secs_f64((1.0 - phase) / 2.0));
    Some(((t * 2.0) as u64).is_multiple_of(2))
}

/// Moving effects: the CRT's rolling bright band and the Cyberpunk HUD sweep.
pub fn paint_motion(ctx: &Context, crt: bool) {
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(LayerId::new(
        egui::Order::Tooltip,
        egui::Id::unique("theme-motion"),
    ));
    let t = now(ctx);
    let mut next: Option<f64> = None;
    if crt {
        let period = 9.0;
        let band = 140.0;
        let f = (t % period) / period;
        let y = rect.top() - band + f as f32 * (rect.height() + 2.0 * band);
        let r = Rect::from_min_max(Pos2::new(rect.left(), y), Pos2::new(rect.right(), y + band));
        vgrad(
            &painter,
            r.with_max_y(r.center().y),
            Color32::TRANSPARENT,
            Color32::from_white_alpha(7),
        );
        vgrad(
            &painter,
            r.with_min_y(r.center().y),
            Color32::from_white_alpha(7),
            Color32::TRANSPARENT,
        );
        next = Some(1.0 / 20.0);
    }
    if theme(current()).fx == Fx::Cyberdeck {
        let period = 8.0;
        let dur = 1.6;
        let f = t % period;
        if f < dur {
            let k = (f / dur) as f32;
            let y = rect.top() + k * rect.height();
            let trail =
                Rect::from_min_max(Pos2::new(rect.left(), y - 60.0), Pos2::new(rect.right(), y));
            vgrad(&painter, trail, Color32::TRANSPARENT, ca(0x00f0ff, 22));
            painter.hline(rect.x_range(), y, Stroke::new(1.5, ca(0x00f0ff, 90)));
            next = Some(next.unwrap_or(1.0).min(1.0 / 30.0));
        } else {
            next = Some(next.unwrap_or(f64::MAX).min(period - f));
        }
    }
    if let Some(s) = next {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(s));
    }
}

// ------------------------------------------------------------------ start-up screen

pub struct Boot {
    start: Option<f64>,
    id: ThemeId,
    roms: u64,
    systems: usize,
}

pub enum BootStep {
    Running,
    Finished,
}

const BOOT_SECS: f32 = 2.6;
const FADE_SECS: f32 = 0.45;

impl Boot {
    pub fn new(id: ThemeId, roms: u64, systems: usize) -> Option<Boot> {
        (id != ThemeId::Standard).then_some(Boot {
            start: None,
            id,
            roms,
            systems,
        })
    }

    /// Draw the start-up screen. Clicking or pressing a key skips it.
    pub fn show(&mut self, ctx: &Context) -> BootStep {
        let t_now = now(ctx);
        let start = *self.start.get_or_insert(t_now);
        let el = (t_now - start) as f32;
        let skip = ctx.input(|i| {
            i.pointer.any_pressed()
                || i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Key { pressed: true, .. }))
        });
        if el > BOOT_SECS + FADE_SECS || (skip && el > 0.1) {
            return BootStep::Finished;
        }
        let rect = ctx.content_rect();
        let t = theme(self.id);
        egui::Area::new(egui::Id::unique("boot-screen"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.allocate_rect(rect, Sense::click());
                let alpha = 1.0 - ((el - BOOT_SECS) / FADE_SECS).clamp(0.0, 1.0);
                let mut painter = ui.painter().clone();
                painter.set_opacity(alpha);
                self.draw(&painter, rect, &t, el);
            });
        ctx.request_repaint();
        BootStep::Running
    }

    fn draw(&self, painter: &Painter, rect: Rect, t: &Theme, el: f32) {
        let p = t.p;
        painter.rect_filled(rect, 0, p.bg.to_opaque());
        let head = |size: f32| FontId::new(size * t.head.head_scale(), t.head.family());
        let body = |size: f32| FontId::new(size * t.body.body_scale(), t.body.family());
        let ctr = rect.center();
        let blink = ((el * 2.5) as u32).is_multiple_of(2);
        let ease = |x: f32| 1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3);
        match self.id {
            ThemeId::GameBoy => {
                let y = rect.top() - 40.0 + (ctr.y - rect.top() + 40.0) * ease(el / 1.3);
                painter.text(
                    Pos2::new(ctr.x, y),
                    Align2::CENTER_CENTER,
                    "Tabularium",
                    head(46.0),
                    p.fg,
                );
                if el > 1.3 {
                    painter.text(
                        Pos2::new(ctr.x, y + 48.0),
                        Align2::CENTER_CENTER,
                        format!("{} ROMS", self.roms),
                        body(15.0),
                        p.dim,
                    );
                }
            }
            ThemeId::AtomicPurple => {
                let word = "Tabularium";
                let colours = [0xff4f6d, 0xffb000, 0x3ec46d, 0x2f9bff, 0xc58bff];
                let font = head(46.0);
                let widths: Vec<f32> = word
                    .chars()
                    .map(|ch| {
                        painter
                            .layout_no_wrap(ch.to_string(), font.clone(), Color32::WHITE)
                            .size()
                            .x
                    })
                    .collect();
                let mut x = ctr.x - widths.iter().sum::<f32>() / 2.0;
                for (i, ch) in word.chars().enumerate() {
                    let shown = el - i as f32 * 0.08;
                    if shown > 0.0 {
                        let hop = (1.0 - ease(shown / 0.25)) * 18.0;
                        painter.text(
                            Pos2::new(x, ctr.y - hop),
                            Align2::LEFT_CENTER,
                            ch.to_string(),
                            font.clone(),
                            c(colours[i % colours.len()]),
                        );
                    }
                    x += widths[i];
                }
                if el > 1.0 {
                    painter.text(
                        ctr + Vec2::new(0.0, 50.0),
                        Align2::CENTER_CENTER,
                        "PRESS START",
                        body(15.0),
                        if blink { p.fg } else { p.dim },
                    );
                }
            }
            ThemeId::Nes | ThemeId::Famicom => {
                painter.text(
                    ctr - Vec2::new(0.0, 30.0),
                    Align2::CENTER_CENTER,
                    "TABULARIUM",
                    head(52.0),
                    p.accent,
                );
                painter.hline(
                    (ctr.x - 180.0)..=(ctr.x + 180.0),
                    ctr.y + 8.0,
                    Stroke::new(3.0, p.accent),
                );
                if el > 0.5 && blink {
                    painter.text(
                        ctr + Vec2::new(0.0, 50.0),
                        Align2::CENTER_CENTER,
                        "PUSH START BUTTON",
                        body(16.0),
                        p.fg,
                    );
                }
                painter.text(
                    Pos2::new(ctr.x, rect.bottom() - 40.0),
                    Align2::CENTER_CENTER,
                    format!("{} ROMS · {} SYSTEMS", self.roms, self.systems),
                    body(13.0),
                    p.dim,
                );
            }
            ThemeId::Commodore64 => {
                painter.rect_filled(rect, 0, p.bar.to_opaque());
                let inner = rect.shrink2(Vec2::new(rect.width() * 0.07, rect.height() * 0.09));
                painter.rect_filled(inner, 0, p.bg.to_opaque());
                let lines = [
                    "    **** TABULARIUM ROM SYSTEM V2 ****".to_string(),
                    String::new(),
                    format!(" {} ROMS ON FILE  {} SYSTEMS", self.roms, self.systems),
                    String::new(),
                    "READY.".to_string(),
                ];
                let font = body(20.0);
                let lh = font.size * 1.15;
                let shown = ((el - 0.2) / 0.22).max(0.0) as usize;
                let mut y = inner.top() + 14.0;
                for l in lines.iter().take(shown.min(lines.len())) {
                    painter.text(
                        Pos2::new(inner.left() + 14.0, y),
                        Align2::LEFT_TOP,
                        l,
                        font.clone(),
                        p.fg,
                    );
                    y += lh;
                }
                if shown >= lines.len() && blink {
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(inner.left() + 14.0, y),
                            Vec2::new(lh * 0.55, lh * 0.85),
                        ),
                        0,
                        p.fg,
                    );
                }
            }
            ThemeId::ArcadeCrt => {
                let f = body(16.0);
                painter.text(
                    Pos2::new(rect.left() + 30.0, rect.top() + 24.0),
                    Align2::LEFT_TOP,
                    "1UP",
                    f.clone(),
                    p.accent,
                );
                painter.text(
                    Pos2::new(rect.left() + 30.0, rect.top() + 46.0),
                    Align2::LEFT_TOP,
                    "00",
                    f.clone(),
                    p.fg,
                );
                painter.text(
                    Pos2::new(ctr.x, rect.top() + 24.0),
                    Align2::CENTER_TOP,
                    "HI-SCORE",
                    f.clone(),
                    p.accent,
                );
                painter.text(
                    Pos2::new(ctr.x, rect.top() + 46.0),
                    Align2::CENTER_TOP,
                    format!("{:06}", self.roms * 100),
                    f.clone(),
                    p.fg,
                );
                let pos = ctr - Vec2::new(0.0, 20.0);
                painter.text(
                    pos + Vec2::new(-3.0, 0.0),
                    Align2::CENTER_CENTER,
                    "TABULARIUM",
                    head(54.0),
                    c(0xff2e88).gamma_multiply(0.6),
                );
                painter.text(
                    pos + Vec2::new(3.0, 0.0),
                    Align2::CENTER_CENTER,
                    "TABULARIUM",
                    head(54.0),
                    c(0x00f0ff).gamma_multiply(0.6),
                );
                painter.text(
                    pos,
                    Align2::CENTER_CENTER,
                    "TABULARIUM",
                    head(54.0),
                    Color32::WHITE,
                );
                if blink {
                    painter.text(
                        ctr + Vec2::new(0.0, 50.0),
                        Align2::CENTER_CENTER,
                        "INSERT COIN",
                        body(20.0),
                        p.accent2,
                    );
                }
                painter.text(
                    Pos2::new(rect.right() - 30.0, rect.bottom() - 30.0),
                    Align2::RIGHT_BOTTOM,
                    "CREDIT 00",
                    f,
                    p.fg,
                );
            }
            ThemeId::VirtualBoy => {
                let hz = ctr.y + 40.0;
                let col = p.accent.gamma_multiply(0.5);
                for i in 0..=16 {
                    let x = rect.left() - rect.width() * 0.5 + i as f32 * rect.width() / 8.0;
                    painter.line_segment(
                        [Pos2::new(ctr.x, hz), Pos2::new(x, rect.bottom())],
                        Stroke::new(1.0, col),
                    );
                }
                let scroll = (el * 0.6).fract();
                for k in 0..10 {
                    let f = ((k as f32 + scroll) / 10.0).powi(2);
                    let y = hz + (rect.bottom() - hz) * f;
                    painter.hline(rect.x_range(), y, Stroke::new(1.0, col));
                }
                let split = (1.0 - ease(el / 1.2)) * 60.0;
                let pos = ctr - Vec2::new(0.0, 50.0);
                painter.text(
                    pos - Vec2::new(split, 0.0),
                    Align2::CENTER_CENTER,
                    "TABULARIUM",
                    head(54.0),
                    p.dim,
                );
                painter.text(
                    pos + Vec2::new(split, 0.0),
                    Align2::CENTER_CENTER,
                    "TABULARIUM",
                    head(54.0),
                    p.fg,
                );
            }
            ThemeId::GameCube => {
                let spin = 7.0 * ease(el / 1.6);
                let (sa, ca_) = spin.sin_cos();
                let tilt: f32 = 0.55;
                let (st, ct) = tilt.sin_cos();
                let proj = |x: f32, y: f32, z: f32| {
                    let (x, z) = (x * ca_ - z * sa, x * sa + z * ca_);
                    let (y, z) = (y * ct - z * st, y * st + z * ct);
                    let _ = z;
                    ctr + Vec2::new(x, -y) * 55.0 - Vec2::new(0.0, 50.0)
                };
                let v: Vec<Pos2> = (0..8)
                    .map(|i| {
                        let f = |b: usize| if i & b != 0 { 1.0 } else { -1.0 };
                        proj(f(1), f(2), f(4))
                    })
                    .collect();
                for (a, b) in [
                    (0, 1),
                    (2, 3),
                    (4, 5),
                    (6, 7),
                    (0, 2),
                    (1, 3),
                    (4, 6),
                    (5, 7),
                    (0, 4),
                    (1, 5),
                    (2, 6),
                    (3, 7),
                ] {
                    painter.line_segment([v[a], v[b]], Stroke::new(3.0, p.accent));
                }
                let a = ((el - 1.2) / 0.5).clamp(0.0, 1.0);
                painter.text(
                    ctr + Vec2::new(0.0, 70.0),
                    Align2::CENTER_CENTER,
                    "Tabularium",
                    head(40.0),
                    p.fg.gamma_multiply(a),
                );
            }
            ThemeId::Cyberpunk => {
                vgrad(painter, rect, c(0x140b28), c(0x05030a));
                let lines = [
                    "> BOOT TABULARIUM.DECK".to_string(),
                    "> CHECKING LIBRARY ............ OK".to_string(),
                    format!(
                        "> {} ROMS INDEXED ACROSS {} SYSTEMS",
                        self.roms, self.systems
                    ),
                    "> NEURAL LINK ESTABLISHED".to_string(),
                    "> SYS ONLINE".to_string(),
                ];
                let font = body(18.0);
                let mut budget = (el * 55.0) as usize;
                let mut y = rect.top() + rect.height() * 0.3;
                let x = rect.left() + rect.width() * 0.2;
                let mut cursor = Pos2::new(x, y);
                for (i, l) in lines.iter().enumerate() {
                    let n = budget.min(l.chars().count());
                    budget -= n;
                    let s: String = l.chars().take(n).collect();
                    let col = if i == lines.len() - 1 {
                        c(0xf3e600)
                    } else {
                        c(0x00f0ff)
                    };
                    let r = painter.text(Pos2::new(x, y), Align2::LEFT_TOP, s, font.clone(), col);
                    cursor = Pos2::new(r.right() + 4.0, y);
                    y += font.size * 1.4;
                    if budget == 0 {
                        break;
                    }
                }
                if blink {
                    painter.rect_filled(
                        Rect::from_min_size(cursor, Vec2::new(10.0, font.size)),
                        0,
                        c(0xf3e600),
                    );
                }
                let bar =
                    Rect::from_min_size(Pos2::new(x, y + 20.0), Vec2::new(rect.width() * 0.6, 8.0));
                painter.rect_stroke(
                    bar,
                    0,
                    Stroke::new(1.0, c(0xff2e88)),
                    egui::StrokeKind::Outside,
                );
                painter.rect_filled(
                    bar.with_max_x(bar.left() + bar.width() * (el / 2.0).min(1.0)),
                    0,
                    c(0xff2e88),
                );
                super::hud_frame(painter, rect.shrink(18.0), c(0x00f0ff), true);
            }
            ThemeId::Standard => {}
        }
    }
}
