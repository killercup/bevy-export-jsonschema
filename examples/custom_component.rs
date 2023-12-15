use bevy::prelude::*;
use bevy_export_jsonschema::ExportTypesExt;
use bevy_inspector_egui::prelude::*;

fn main() {
    let mut app = App::new();
    app.register_type::<Player>();
    app.export_types(std::io::stdout());
}

#[derive(Reflect, Component, Default, InspectorOptions)]
#[reflect(Component, InspectorOptions)]
struct Player {
    name: String,
    #[inspector(min = 0.0, max = 1.0)]
    health: f32,
}
