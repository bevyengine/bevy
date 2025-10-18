//! BSN Template for loading images and displaying them as [`ImageNodes`].
use bevy_asset::AssetServer;
use bevy_ecs::template::template;
use bevy_scene2::{bsn, Scene};
use bevy_ui::{widget::ImageNode, Node, Val};

/// Template which displays an icon.
pub fn icon(image: &'static str) -> impl Scene {
    bsn! {
        Node {
            height: Val::Px(14.0),
        }
        template(move |entity| {
            let handle = entity.resource::<AssetServer>().load(image);
            Ok(ImageNode::new(handle))
        })
    }
}
