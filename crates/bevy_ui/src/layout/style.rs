use core::{marker::PhantomData, slice};

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Component;
use taffy::{
    geometry::{Line, Point, Rect, Size},
    style::{
        AlignContent, AlignItems, AlignSelf, BlockContainerStyle, BlockItemStyle,
        BoxGenerationMode, BoxSizing, CoreStyle, Dimension, Direction, Display, FlexDirection,
        FlexWrap, FlexboxContainerStyle, FlexboxItemStyle, GenericGridTemplateComponent,
        GenericRepetition, GridAutoFlow, GridContainerStyle, GridItemStyle,
        GridPlacement as TaffyGridPlacement, GridTemplateArea, JustifyContent, LengthPercentage,
        LengthPercentageAuto, Overflow, Position, RepetitionCount, Style, TemplateLineNames,
        TextAlign, TrackSizingFunction,
    },
};

use crate::{GridTrack, GridTrackRepetition, LayoutContext, Node, RepeatedGridTrack};

pub static VIEWPORT_NODE: Node = Node::VIEWPORT;

#[expect(
    unsafe_code,
    reason = "taffy::Style is only thread-unsafe with the calc feature"
)]
unsafe impl Send for TaffyStyle {}

#[expect(
    unsafe_code,
    reason = "taffy::Style is only thread-unsafe with the calc feature"
)]
unsafe impl Sync for TaffyStyle {}

#[derive(Default, Component, Clone, Deref, DerefMut)]
pub struct TaffyStyle(pub Style);

/// Style adapter exposing Bevy [`Node`]s through Taffy's style traits.
#[derive(Clone)]
pub(super) struct NodeStyle<'a> {
    pub node: &'a Node,
    pub(crate) context: LayoutContext,
}

#[derive(Clone)]
pub(super) struct GridTemplateTrackList<'a> {
    tracks: slice::Iter<'a, RepeatedGridTrack>,
    context: LayoutContext,
}

impl<'a> Iterator for GridTemplateTrackList<'a> {
    type Item = GenericGridTemplateComponent<String, RepeatedGridTrackRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let track = self.tracks.next()?;
        Some(
            if track.tracks.len() == 1 && track.repetition == GridTrackRepetition::Count(1) {
                GenericGridTemplateComponent::Single(
                    track.tracks[0].into_taffy_track(&self.context),
                )
            } else {
                GenericGridTemplateComponent::Repeat(RepeatedGridTrackRef {
                    track,
                    context: self.context,
                })
            },
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tracks.size_hint()
    }
}

impl ExactSizeIterator for GridTemplateTrackList<'_> {}

#[derive(Clone)]
pub(super) struct GridTrackList<'a> {
    tracks: slice::Iter<'a, GridTrack>,
    context: LayoutContext,
}

impl Iterator for GridTrackList<'_> {
    type Item = TrackSizingFunction;

    fn next(&mut self) -> Option<Self::Item> {
        self.tracks
            .next()
            .map(|track| track.into_taffy_track(&self.context))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.tracks.size_hint()
    }
}

impl ExactSizeIterator for GridTrackList<'_> {}

#[derive(Copy, Clone)]
pub(super) struct RepeatedGridTrackRef<'a> {
    track: &'a RepeatedGridTrack,
    context: LayoutContext,
}

impl GenericRepetition for RepeatedGridTrackRef<'_> {
    type CustomIdent = String;

    type RepetitionTrackList<'a>
        = GridTrackList<'a>
    where
        Self: 'a;

    type TemplateLineNames<'a>
        = EmptyLineNames<'a>
    where
        Self: 'a;

    fn count(&self) -> RepetitionCount {
        match self.track.repetition {
            GridTrackRepetition::Count(count) => RepetitionCount::Count(count),
            GridTrackRepetition::AutoFill => RepetitionCount::AutoFill,
            GridTrackRepetition::AutoFit => RepetitionCount::AutoFit,
        }
    }

    fn tracks(&self) -> Self::RepetitionTrackList<'_> {
        GridTrackList {
            tracks: self.track.tracks.iter(),
            context: self.context,
        }
    }

    fn lines_names(&self) -> Self::TemplateLineNames<'_> {
        EmptyLineNames::new()
    }
}

#[derive(Copy, Clone)]
pub(super) struct EmptyLineNames<'a> {
    marker: PhantomData<&'a String>,
}

impl<'a> EmptyLineNames<'a> {
    const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<'a> Iterator for EmptyLineNames<'a> {
    type Item = EmptyLineNameSet<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

impl ExactSizeIterator for EmptyLineNames<'_> {}

impl<'a> TemplateLineNames<'a, String> for EmptyLineNames<'a> {
    type LineNameSet<'b>
        = EmptyLineNameSet<'b>
    where
        Self: 'b;
}

#[derive(Copy, Clone)]
pub(super) struct EmptyLineNameSet<'a> {
    marker: PhantomData<&'a String>,
}

impl<'a> Iterator for EmptyLineNameSet<'a> {
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

impl ExactSizeIterator for EmptyLineNameSet<'_> {}

impl<'a> NodeStyle<'a> {
    pub(super) fn from_node(node: &'a Node, context: LayoutContext) -> Self {
        Self { node, context }
    }
    pub(super) fn display(&self) -> Display {
        self.node().display.into()
    }

    fn node(&self) -> &Node {
        self.node
    }
}

impl CoreStyle for NodeStyle<'_> {
    type CustomIdent = String;

    #[inline(always)]
    fn box_generation_mode(&self) -> BoxGenerationMode {
        match self.display() {
            Display::None => BoxGenerationMode::None,
            _ => BoxGenerationMode::Normal,
        }
    }

    #[inline(always)]
    fn is_block(&self) -> bool {
        matches!(self.display(), Display::Block)
    }

    #[inline(always)]
    fn box_sizing(&self) -> BoxSizing {
        self.node().box_sizing.into()
    }

    #[inline(always)]
    fn direction(&self) -> Direction {
        self.node().direction.into()
    }

    #[inline(always)]
    fn overflow(&self) -> Point<Overflow> {
        Point {
            x: self.node().overflow.x.into(),
            y: self.node().overflow.y.into(),
        }
    }

    #[inline(always)]
    fn scrollbar_width(&self) -> f32 {
        self.node().scrollbar_width * self.context.scale_factor
    }

    #[inline(always)]
    fn position(&self) -> Position {
        self.node().position_type.into()
    }

    #[inline(always)]
    fn inset(&self) -> Rect<LengthPercentageAuto> {
        Rect {
            left: self.node().left.into_length_percentage_auto(&self.context),
            right: self.node().right.into_length_percentage_auto(&self.context),
            top: self.node().top.into_length_percentage_auto(&self.context),
            bottom: self
                .node()
                .bottom
                .into_length_percentage_auto(&self.context),
        }
    }

    #[inline(always)]
    fn size(&self) -> Size<Dimension> {
        Size {
            width: self.node().width.into_dimension(&self.context),
            height: self.node().height.into_dimension(&self.context),
        }
    }

    #[inline(always)]
    fn min_size(&self) -> Size<LengthPercentageAuto> {
        Size {
            width: self
                .node()
                .min_width
                .into_length_percentage_auto(&self.context),
            height: self
                .node()
                .min_height
                .into_length_percentage_auto(&self.context),
        }
    }

    #[inline(always)]
    fn max_size(&self) -> Size<LengthPercentageAuto> {
        Size {
            width: self
                .node()
                .max_width
                .into_length_percentage_auto(&self.context),
            height: self
                .node()
                .max_height
                .into_length_percentage_auto(&self.context),
        }
    }

    #[inline(always)]
    fn aspect_ratio(&self) -> Option<f32> {
        self.node().aspect_ratio
    }

    #[inline(always)]
    fn margin(&self) -> Rect<LengthPercentageAuto> {
        self.node()
            .margin
            .map_to_taffy_rect(|margin| margin.into_length_percentage_auto(&self.context))
    }

    #[inline(always)]
    fn padding(&self) -> Rect<LengthPercentage> {
        self.node()
            .padding
            .map_to_taffy_rect(|padding| padding.into_length_percentage(&self.context))
    }

    #[inline(always)]
    fn border(&self) -> Rect<LengthPercentage> {
        self.node()
            .border
            .map_to_taffy_rect(|border| border.into_length_percentage(&self.context))
    }
}

impl BlockContainerStyle for NodeStyle<'_> {
    #[inline(always)]
    fn text_align(&self) -> TextAlign {
        TextAlign::Auto
    }

    fn align_content(&self) -> Option<AlignContent> {
        self.node().align_content.into()
    }
}

// Doesn't need anything, we don't support tables or float layout.
impl BlockItemStyle for NodeStyle<'_> {}

impl FlexboxContainerStyle for NodeStyle<'_> {
    #[inline(always)]
    fn flex_direction(&self) -> FlexDirection {
        self.node().flex_direction.into()
    }

    #[inline(always)]
    fn flex_wrap(&self) -> FlexWrap {
        self.node().flex_wrap.into()
    }

    #[inline(always)]
    fn gap(&self) -> Size<LengthPercentage> {
        Size {
            width: self.node().column_gap.into_length_percentage(&self.context),
            height: self.node().row_gap.into_length_percentage(&self.context),
        }
    }

    #[inline(always)]
    fn align_content(&self) -> Option<AlignContent> {
        self.node().align_content.into()
    }

    #[inline(always)]
    fn align_items(&self) -> Option<AlignItems> {
        self.node().align_items.into()
    }

    #[inline(always)]
    fn justify_content(&self) -> Option<JustifyContent> {
        self.node().justify_content.into()
    }
}

impl FlexboxItemStyle for NodeStyle<'_> {
    #[inline(always)]
    fn flex_basis(&self) -> Dimension {
        self.node().flex_basis.into_dimension(&self.context)
    }

    #[inline(always)]
    fn flex_grow(&self) -> f32 {
        self.node().flex_grow
    }

    #[inline(always)]
    fn flex_shrink(&self) -> f32 {
        self.node().flex_shrink
    }

    #[inline(always)]
    fn align_self(&self) -> Option<AlignSelf> {
        self.node().align_self.into()
    }
}

impl GridContainerStyle for NodeStyle<'_> {
    type Repetition<'a>
        = RepeatedGridTrackRef<'a>
    where
        Self: 'a;

    type TemplateTrackList<'a>
        = GridTemplateTrackList<'a>
    where
        Self: 'a;

    type AutoTrackList<'a>
        = GridTrackList<'a>
    where
        Self: 'a;

    type TemplateLineNames<'a>
        = EmptyLineNames<'a>
    where
        Self: 'a;

    type GridTemplateAreas<'a>
        = core::iter::Empty<GridTemplateArea<String>>
    where
        Self: 'a;

    #[inline(always)]
    fn grid_template_rows(&self) -> Option<Self::TemplateTrackList<'_>> {
        Some(GridTemplateTrackList {
            tracks: self.node().grid_template_rows.iter(),
            context: self.context,
        })
    }

    #[inline(always)]
    fn grid_template_columns(&self) -> Option<Self::TemplateTrackList<'_>> {
        Some(GridTemplateTrackList {
            tracks: self.node().grid_template_columns.iter(),
            context: self.context,
        })
    }

    #[inline(always)]
    fn grid_auto_rows(&self) -> Self::AutoTrackList<'_> {
        GridTrackList {
            tracks: self.node().grid_auto_rows.iter(),
            context: self.context,
        }
    }

    #[inline(always)]
    fn grid_auto_columns(&self) -> Self::AutoTrackList<'_> {
        GridTrackList {
            tracks: self.node().grid_auto_columns.iter(),
            context: self.context,
        }
    }

    #[inline(always)]
    fn grid_template_areas(&self) -> Option<Self::GridTemplateAreas<'_>> {
        None
    }

    #[inline(always)]
    fn grid_template_column_names(&self) -> Option<Self::TemplateLineNames<'_>> {
        None
    }

    #[inline(always)]
    fn grid_template_row_names(&self) -> Option<Self::TemplateLineNames<'_>> {
        None
    }

    #[inline(always)]
    fn grid_auto_flow(&self) -> GridAutoFlow {
        self.node().grid_auto_flow.into()
    }

    #[inline(always)]
    fn gap(&self) -> Size<LengthPercentage> {
        Size {
            width: self.node().column_gap.into_length_percentage(&self.context),
            height: self.node().row_gap.into_length_percentage(&self.context),
        }
    }

    #[inline(always)]
    fn align_content(&self) -> Option<AlignContent> {
        self.node().align_content.into()
    }

    #[inline(always)]
    fn justify_content(&self) -> Option<JustifyContent> {
        self.node().justify_content.into()
    }

    #[inline(always)]
    fn align_items(&self) -> Option<AlignItems> {
        self.node().align_items.into()
    }

    #[inline(always)]
    fn justify_items(&self) -> Option<AlignItems> {
        self.node().justify_items.into()
    }
}

impl GridItemStyle for NodeStyle<'_> {
    #[inline(always)]
    fn grid_row(&self) -> Line<TaffyGridPlacement<String>> {
        self.node().grid_row.into()
    }

    #[inline(always)]
    fn grid_column(&self) -> Line<TaffyGridPlacement<String>> {
        self.node().grid_column.into()
    }

    #[inline(always)]
    fn align_self(&self) -> Option<AlignSelf> {
        self.node().align_self.into()
    }

    #[inline(always)]
    fn justify_self(&self) -> Option<AlignSelf> {
        self.node().justify_self.into()
    }
}
