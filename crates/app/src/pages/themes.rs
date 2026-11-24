use crate::app::RomApp;
use crate::themes::{self, ThemeId};
use crate::widgets::{card, page_title, section};
use egui::{RichText, Ui};

impl RomApp {
    pub fn themes_page(&mut self, ui: &mut Ui) {
        page_title(
            ui,
            "Themes",
            "Change how Tabularium looks. Your choice is remembered.",
        );
        let current = self.settings.theme;
        let mut chosen = None;
        let width = ui.available_width();
        let cols = ((width / 260.0).floor() as usize).clamp(1, 4);
        let card_w = (width - (cols as f32 - 1.0) * 12.0) / cols as f32 - 2.0;
        egui::Grid::new("theme-grid")
            .num_columns(cols)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                for (i, id) in ThemeId::ALL.iter().enumerate() {
                    let t = themes::info(*id);
                    let selected = *id == current;
                    let stroke = if selected {
                        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill)
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke
                    };
                    let resp = egui::Frame::group(ui.style())
                        .stroke(stroke)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(card_w - 24.0);
                                ui.horizontal(|ui| {
                                    for col in themes::swatches(*id) {
                                        let (r, _) = ui.allocate_exact_size(
                                            egui::vec2(26.0, 18.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().rect_filled(r, 2, col);
                                        ui.painter().rect_stroke(
                                            r,
                                            2,
                                            egui::Stroke::new(
                                                1.0,
                                                egui::Color32::from_black_alpha(60),
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                });
                                ui.add_space(4.0);
                                let name = if selected {
                                    format!("✔ {}", t.name)
                                } else {
                                    t.name.to_string()
                                };
                                ui.label(RichText::new(name).strong());
                                ui.add(egui::Label::new(RichText::new(t.blurb).weak()).wrap());
                            });
                        })
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if resp.clicked() {
                        chosen = Some(*id);
                    }
                    if i % cols == cols - 1 {
                        ui.end_row();
                    }
                }
            });
        if let Some(id) = chosen {
            self.settings.theme = id;
            (self.settings.crt_effect, self.settings.lcd_effect) = themes::default_effects(id);
            themes::apply(ui.ctx(), id, &self.base_styles);
            self.view.invalidate();
        }

        ui.add_space(16.0);
        card(ui, |ui| {
            section(ui, "Screen effects");
            ui.label(
                RichText::new("These work with any theme. Picking a theme sets them to suit it; change them here afterwards.")
                    .weak(),
            );
            ui.checkbox(&mut self.settings.crt_effect, "CRT scanlines")
                .on_hover_text(
                "Horizontal scanlines and darkened corners, like an old tube TV or arcade monitor.",
            );
            ui.checkbox(&mut self.settings.lcd_effect, "LCD screen")
                .on_hover_text(
                "Red-green-blue subpixel stripes, gaps between pixel rows and a faint glass glare.",
            );
        });

        ui.add_space(12.0);
        card(ui, |ui| {
            section(ui, "Motion");
            ui.checkbox(&mut self.settings.animations, "Animations")
                .on_hover_text(
                    "Blinking cursors, the CRT's rolling band and the Cyberpunk scanner sweep.",
                );
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.settings.boot_screen, "Start-up screen")
                    .on_hover_text("A short intro in the theme's style when Tabularium opens. Click or press a key to skip it.");
                if ui.button("Play it now").clicked() {
                    let roms = self.data.stats.values().map(|s| s.count).sum();
                    self.boot = themes::Boot::new(self.settings.theme, roms, self.data.stats.len());
                }
            });
        });
    }
}
