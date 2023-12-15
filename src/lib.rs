use std::{collections::HashMap, io::Write};

use bevy_app::App;
use bevy_ecs::reflect::{AppTypeRegistry, ReflectComponent, ReflectResource};
use bevy_reflect::{TypeInfo, TypeRegistration, VariantInfo};
use serde_json::{json, Value};

pub trait ExportTypesExt {
    fn export_types(&mut self, writer: impl Write);
}

impl ExportTypesExt for App {
    fn export_types(&mut self, writer: impl Write) {
        let types = self.world.resource_mut::<AppTypeRegistry>();
        let types = types.read();
        let mut schemas = types.iter().map(export_type).collect::<Vec<_>>();
        schemas.sort_by_key(|t| t.get("name").unwrap().as_str().unwrap().to_string());

        serde_json::to_writer_pretty(
            writer,
            &json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "bevy game schema",
                "oneOf": schemas,
            }),
        )
        .expect("valid json");

        eprintln!("wrote schema containing {} types", schemas.len());
    }
}

pub fn export_type(reg: &TypeRegistration) -> Value {
    let t = reg.type_info();
    let mut schema = match t {
        TypeInfo::Struct(info) => {
            let properties = info
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    (
                        field.name(),
                        add_min_max(json!({ "type": field.type_path() }), reg, idx, None),
                    )
                })
                .collect::<HashMap<_, _>>();

            json!({
                "type": "object",
                "name": t.type_path(),
                "properties": properties,
                "additionalProperties": false,
                "required": info
                    .iter()
                    .filter(|field| !field.type_path().starts_with("core::option::Option"))
                    .map(|field| field.name())
                    .collect::<Vec<_>>(),
            })
        }
        TypeInfo::Enum(info) => {
            let simple = info
                .iter()
                .all(|variant| matches!(variant, VariantInfo::Unit(_)));
            if simple {
                json!({
                    "type": "string",
                    "name": t.type_path(),
                    "enum": info
                        .iter()
                        .map(|variant| match variant {
                            VariantInfo::Unit(v) => v.name(),
                            _ => unreachable!(),
                        })
                        .collect::<Vec<_>>(),
                })
            } else {
                let variants = info
                .iter()
                .enumerate()
                .map(|(field_idx, variant)| match variant {
                    VariantInfo::Struct(v) => json!({
                        "type": "object",
                        "name": t.type_path(),
                        "properties": v
                            .iter()
                            .enumerate()
                            .map(|(variant_idx, field)| (field.name(), add_min_max(json!({"type": field.type_path(), "name": field.name()}), reg, field_idx, Some(variant_idx))))
                            .collect::<HashMap<_, _>>(),
                        "additionalProperties": false,
                        "required": v
                            .iter()
                            .filter(|field| field.type_path().starts_with("core::option::Option"))
                            .map(|field| field.name())
                            .collect::<Vec<_>>(),
                    }),
                    VariantInfo::Tuple(v) => json!({
                        "type": "array",
                        "prefixItems": v
                            .iter()
                            .enumerate()
                            .map(|(variant_idx, field)| add_min_max(json!({"type": field.type_path()}), reg, field_idx, Some(variant_idx)))
                            .collect::<Vec<_>>(),
                        "items": false,
                    }),
                    VariantInfo::Unit(v) => json!({
                        "const": v.name(),
                    }),
                })
                .collect::<Vec<_>>();

                json!({
                    "type": "object",
                    "name": t.type_path(),
                    "oneOf": variants,
                })
            }
        }
        TypeInfo::TupleStruct(info) => json!({
            "name": t.type_path(),
            "type": "array",
            "prefixItems": info
                .iter()
                .enumerate()
                .map(|(idx, field)| add_min_max(json!({"type": field.type_path()}), reg, idx, None))
                .collect::<Vec<_>>(),
            "items": false,
        }),
        TypeInfo::List(info) => {
            json!({
                "name": t.type_path(),
                "type": "array",
                "items": json!({"type": info.type_path()}),
            })
        }
        TypeInfo::Array(info) => json!({
            "name": t.type_path(),
            "type": "array",
            "items": json!({"type": info.type_path()}),
        }),
        TypeInfo::Map(info) => json!({
            "name": t.type_path(),
            "type": "object",
            "additionalProperties": json!({"type": info.type_path()}),
        }),
        TypeInfo::Tuple(info) => json!({
            "name": t.type_path(),
            "type": "array",
            "prefixItems": info
                .iter()
                .enumerate()
                .map(|(idx, field)| add_min_max(json!({"type": field.type_path()}), reg, idx, None))
                .collect::<Vec<_>>(),
            "items": false,
        }),
        TypeInfo::Value(info) => json!({
            "name": t.type_path(),
            "type": info.type_path(),
        }),
    };
    schema.as_object_mut().unwrap().insert(
        "isComponent".to_owned(),
        reg.data::<ReflectComponent>().is_some().into(),
    );
    schema.as_object_mut().unwrap().insert(
        "isResource".to_owned(),
        reg.data::<ReflectResource>().is_some().into(),
    );
    schema
}

fn add_min_max(
    mut val: Value,
    reg: &TypeRegistration,
    field_index: usize,
    variant_index: Option<usize>,
) -> Value {
    #[cfg(feature = "support-inspector")]
    fn get_min_max(
        reg: &TypeRegistration,
        field_index: usize,
        variant_index: Option<usize>,
    ) -> Option<(Option<f32>, Option<f32>)> {
        use bevy_inspector_egui::inspector_options::{
            std_options::NumberOptions, ReflectInspectorOptions, Target,
        };

        reg.data::<ReflectInspectorOptions>()
            .and_then(|ReflectInspectorOptions(o)| {
                o.get(if let Some(variant_index) = variant_index {
                    Target::VariantField {
                        variant_index,
                        field_index,
                    }
                } else {
                    Target::Field(field_index)
                })
            })
            .and_then(|o| o.downcast_ref::<NumberOptions<f32>>())
            .map(|num| (num.min, num.max))
    }

    #[cfg(not(feature = "support-inspector"))]
    fn get_min_max(
        _reg: &TypeRegistration,
        _field_index: usize,
        _variant_index: Option<usize>,
    ) -> Option<(Option<f32>, Option<f32>)> {
        None
    }

    let Some((min, max)) = get_min_max(reg, field_index, variant_index) else {
        return val;
    };
    let obj = val.as_object_mut().unwrap();
    if let Some(min) = min {
        obj.insert("minimum".to_owned(), min.into());
    }
    if let Some(max) = max {
        obj.insert("maximum".to_owned(), max.into());
    }
    val
}
