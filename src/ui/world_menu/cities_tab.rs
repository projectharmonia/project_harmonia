use bevy_egui::egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId, Ui};

pub(super) struct CitiesTab<'a> {
    cities: &'a [&'static str],
}

impl<'a> CitiesTab<'a> {
    #[must_use]
    pub(super) fn new(cities: &'a [&'static str]) -> Self {
        Self { cities }
    }
}

impl CitiesTab<'_> {
    pub(super) fn show(self, ui: &mut Ui) {
        for family in self.cities {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        Image::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]),
                    );
                    ui.label(*family);
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        if ui.button("✏ Edit").clicked() {}
                    })
                });
            });
        }
    }
}
