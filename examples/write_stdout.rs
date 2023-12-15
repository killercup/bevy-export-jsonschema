use bevy::prelude::*;
use bevy_export_jsonschema::ExportTypesExt;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.export_types(std::io::stdout());
}
