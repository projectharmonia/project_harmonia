use bevy_egui::egui::{epaint::WHITE_UV, Align, Image, Layout, TextureId, Ui};

pub(super) struct FamiliesTab<'a> {
    families: &'a [&'static str],
}

impl<'a> FamiliesTab<'a> {
    #[must_use]
    pub(super) fn new(families: &'a [&'static str]) -> Self {
        Self { families }
    }
}

impl FamiliesTab<'_> {
    pub(super) fn show(self, ui: &mut Ui) {
        for family in self.families {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        Image::new(TextureId::Managed(0), (64.0, 64.0)).uv([WHITE_UV, WHITE_UV]),
                    );
                    ui.label(*family);
                    ui.with_layout(Layout::top_down(Align::Max), |ui| {
                        if ui.button("⏵ Play").clicked() {}
                    })
                });
            });
        }
    }
}
