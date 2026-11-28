//! Middle-button autoscroll, as in web browsers on Windows.
//!
//! Hold the middle button (or the scroll wheel) and move the mouse: whatever
//! is under the pointer scrolls, faster the further the pointer is from where
//! the button went down. Release to stop. A quick middle-click without moving
//! turns it on until the next click.
//!
//! It works by adding ordinary scroll-wheel events to each frame's input, so
//! every scrollable area responds without knowing about it.

use egui::{
    Color32, Context, CursorIcon, Event, LayerId, MouseWheelSource, MouseWheelUnit, Order, Pos2,
    RawInput, Stroke, TouchPhase, Vec2,
};

/// Pointer distance from the anchor that does nothing, so it doesn't drift.
const DEAD_ZONE: f32 = 12.0;
/// Scroll speed (points per second) per point of distance past the dead zone.
const SPEED: f32 = 9.0;
const MAX_SPEED: f32 = 6000.0;
/// A press shorter than this, without moving, toggles autoscroll on.
const CLICK_SECS: f64 = 0.3;

struct Active {
    anchor: Pos2,
    started: f64,
    moved: bool,
    /// Stays on after the button is released (started by a quick click).
    sticky: bool,
}

#[derive(Default)]
pub struct Autoscroll {
    active: Option<Active>,
    pointer: Option<Pos2>,
    last_time: Option<f64>,
    /// The button whose press stopped autoscroll: its press and release are
    /// kept from the app, so stopping doesn't also click whatever is below.
    swallow: Option<egui::PointerButton>,
}

impl Autoscroll {
    /// Watch the raw input for the middle button and add scroll events while active.
    pub fn process(&mut self, ctx: &Context, raw: &mut RawInput) {
        let now = raw.time.unwrap_or(0.0);
        let dt = self
            .last_time
            .map(|t| (now - t) as f32)
            .filter(|d| *d > 0.0)
            .unwrap_or(raw.predicted_dt)
            .min(0.1);
        self.last_time = Some(now);

        let mut kept = Vec::with_capacity(raw.events.len());
        for ev in std::mem::take(&mut raw.events) {
            let mut keep = true;
            match &ev {
                Event::PointerMoved(p) => {
                    self.pointer = Some(*p);
                    if let Some(a) = &mut self.active {
                        if (*p - a.anchor).length() > DEAD_ZONE {
                            a.moved = true;
                        }
                    }
                }
                Event::PointerGone => self.pointer = None,
                Event::PointerButton {
                    button,
                    pressed,
                    pos,
                    ..
                } => match (button, pressed, &mut self.active) {
                    // Any press while it's on (sticky) turns it off, and
                    // goes no further.
                    (b, true, Some(a)) if a.sticky => {
                        self.active = None;
                        self.swallow = Some(*b);
                        keep = false;
                    }
                    (b, false, _) if self.swallow == Some(*b) => {
                        self.swallow = None;
                        keep = false;
                    }
                    (egui::PointerButton::Middle, true, None) => {
                        self.active = Some(Active {
                            anchor: *pos,
                            started: now,
                            moved: false,
                            sticky: false,
                        });
                        self.pointer = Some(*pos);
                    }
                    (egui::PointerButton::Middle, false, Some(a)) if !a.sticky => {
                        if !a.moved && now - a.started < CLICK_SECS {
                            a.sticky = true;
                        } else {
                            self.active = None;
                        }
                    }
                    _ => {}
                },
                Event::WindowFocused(false) => self.active = None,
                _ => {}
            }
            if keep {
                kept.push(ev);
            }
        }
        raw.events = kept;

        let (Some(a), Some(p)) = (&self.active, self.pointer) else {
            return;
        };
        let speed = |d: f32| {
            let past = d.abs() - DEAD_ZONE;
            if past <= 0.0 {
                0.0
            } else {
                (past * SPEED).min(MAX_SPEED) * d.signum()
            }
        };
        let off = p - a.anchor;
        let v = Vec2::new(speed(off.x), speed(off.y));
        if v != Vec2::ZERO {
            // Moving the pointer down reveals content further down, which
            // the wheel event expresses as moving the content up.
            raw.events.push(Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                delta: -v * dt,
                phase: TouchPhase::Move,
                source: MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            });
        }
        ctx.request_repaint();
    }

    /// Draw the anchor marker and set the cursor. Call at the end of each frame.
    pub fn paint(&self, ctx: &Context) {
        let Some(a) = &self.active else { return };
        ctx.set_cursor_icon(CursorIcon::AllScroll);
        let painter = ctx.layer_painter(LayerId::new(
            Order::Foreground,
            egui::Id::unique("autoscroll"),
        ));
        let c = a.anchor;
        painter.circle(
            c,
            14.0,
            Color32::from_black_alpha(140),
            Stroke::new(1.5, Color32::from_white_alpha(200)),
        );
        let col = Color32::from_white_alpha(220);
        for dir in [-1.0f32, 1.0] {
            let tip = c + Vec2::new(0.0, 9.0 * dir);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    tip,
                    tip + Vec2::new(-4.0, -4.5 * dir),
                    tip + Vec2::new(4.0, -4.5 * dir),
                ],
                col,
                Stroke::NONE,
            ));
        }
        painter.circle_filled(c, 2.0, col);
    }
}
