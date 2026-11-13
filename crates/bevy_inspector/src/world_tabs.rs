//! Tabs for the inspector's world views.

use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    reflect::ReflectComponent,
    system::Query,
    template::OptionTemplate,
};
use bevy_feathers::{
    controls::{FeathersTab, FeathersTabList},
    display::caption,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, on, Scene};
use bevy_ui::{px, AlignItems, Display, FlexDirection, Node};
use bevy_ui_widgets::{tablist_self_update, ValueChange};

use crate::{entity_tree::entity_tree_body, resource_list::resource_list_body};

/// A page of [`world_tabs`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum InspectorPage {
    /// The entity tree.
    #[default]
    Entities,
    /// The resource list.
    Resources,
}

/// A tab of [`world_tabs`], and the page it shows.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorPageTab(pub InspectorPage);

/// A page body of [`world_tabs`], shown while its tab is selected.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct InspectorPageBody(pub InspectorPage);

/// A tab strip with one page per [`InspectorPage`].
pub fn world_tabs() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            min_height: px(0),
        }
        Children [
            @FeathersTabList {
                @selected: OptionTemplate::Some(#entities_tab),
            }
            on(tablist_self_update)
            on(show_selected_page)
            Children [
                #entities_tab
                @FeathersTab {
                    @caption: bsn! { @caption("Entities") },
                }
                InspectorPageTab(InspectorPage::Entities)
                --
                @FeathersTab {
                    @caption: bsn! { @caption("Resources") },
                }
                InspectorPageTab(InspectorPage::Resources)
            ]
            --
            @entity_tree_body()
            InspectorPageBody(InspectorPage::Entities)
            --
            @resource_list_body()
            InspectorPageBody(InspectorPage::Resources)
            Node {
                display: Display::None,
            }
        ]
    }
}

/// Observer that shows the page of the selected tab of a [`world_tabs`] strip and hides the others.
pub fn show_selected_page(
    change: On<ValueChange<Option<Entity>>>,
    tabs: Query<&InspectorPageTab>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    mut pages: Query<(&InspectorPageBody, &mut Node)>,
) {
    let Some(InspectorPageTab(page)) = change.value.and_then(|tab| tabs.get(tab).ok()) else {
        return;
    };
    let Ok(parent) = parents.get(change.source) else {
        return;
    };
    let Ok(siblings) = children.get(parent.parent()) else {
        return;
    };
    for sibling in siblings.iter().copied() {
        if let Ok((body, mut node)) = pages.get_mut(sibling) {
            node.display = if body.0 == *page {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}
