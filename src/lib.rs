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
                .map(|field| (field.name(), json!({"type": field.type_path()})))
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
                .map(|variant| match variant {
                    VariantInfo::Struct(v) => json!({
                        "type": "object",
                        "name": t.type_path(),
                        "properties": v
                            .iter()
                            .map(|field| (field.name(), json!({"type": field.type_path(), "name": field.name()})))
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
                            .map(|field| json!({"type": field.type_path()}))
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
                .map(|field| json!({"type": field.type_path()}))
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
                .map(|field| json!({"type": field.type_path()}))
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
