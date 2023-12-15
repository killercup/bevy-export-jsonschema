use bevy::prelude::*;
use bevy_export_jsonschema::ExportTypesExt;
use bevy_xpbd_3d::plugins::PhysicsPlugins;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(PhysicsPlugins::default());
    app.export_types(std::io::stdout());
}
