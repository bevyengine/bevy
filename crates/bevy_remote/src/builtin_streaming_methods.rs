use bevy_ecs::{reflect::AppTypeRegistry, system::In, world::World};
use serde_json::Value;

use crate::{builtin_methods::BrpGetParams, BrpResult};

/// The method path for a `bevy/get+stream` request.
pub const BRP_GET_METHOD: &str = "bevy/get+stream";

/// Handles a `bevy/get` request coming from a client.
pub fn process_remote_streaming_get_request(In(params): In<Option<Value>>, world: &World) -> Option<BrpResult> {
    let BrpGetParams {
        entity,
        components,
        strict,
    } = parse_some(params)?;

    let app_type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = app_type_registry.read();
    let entity_ref = get_entity(world, entity)?;

    let mut response = if strict {
        BrpGetResponse::Strict(Default::default())
    } else {
        BrpGetResponse::Lenient {
            components: Default::default(),
            errors: Default::default(),
        }
    };

    for component_path in components {
        match handle_get_component(&component_path, entity, entity_ref, &type_registry) {
            Ok(serialized_object) => match response {
                BrpGetResponse::Strict(ref mut components)
                | BrpGetResponse::Lenient {
                    ref mut components, ..
                } => {
                    components.extend(serialized_object.into_iter());
                }
            },
            Err(err) => match response {
                BrpGetResponse::Strict(_) => return Err(err),
                BrpGetResponse::Lenient { ref mut errors, .. } => {
                    let err_value = serde_json::to_value(err).map_err(BrpError::internal)?;
                    errors.insert(component_path, err_value);
                }
            },
        }
    }

    serde_json::to_value(response).map_err(BrpError::internal)
}

fn parse_some(params: Option<Value>) -> _ {
    todo!()
}
