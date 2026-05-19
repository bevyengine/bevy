use core::slice;

use taffy::{
    geometry::{Line, Point, Rect, Size},
    style::{
        AlignContent, AlignItems, AlignSelf, BlockContainerStyle, BlockItemStyle,
        BoxGenerationMode, BoxSizing, CoreStyle, Dimension, Direction, Display, FlexDirection,
        FlexWrap, FlexboxContainerStyle, FlexboxItemStyle, GenericGridTemplateComponent,
        GridAutoFlow, GridContainerStyle, GridItemStyle, GridPlacement as TaffyGridPlacement,
        GridTemplateArea, GridTemplateComponent, GridTemplateRepetition, JustifyContent,
        LengthPercentage, LengthPercentageAuto, Overflow, Position, Style, TextAlign,
        TrackSizingFunction,
    },
};

use crate::{
    layout::convert, AlignItems as UiAlignItems, Display as UiDisplay,
    JustifyItems as UiJustifyItems, LayoutContext, Node, Val,
};

fn as_component_ref(
    component: &GridTemplateComponent<String>,
) -> GenericGridTemplateComponent<String, &GridTemplateRepetition<String>> {
    component.as_component_ref()
}

/// Runtime style adapter that exposes Bevy [`Node`] values through Taffy's style traits.
#[derive(Clone)]
pub(super) struct CoreNode {
    node: Node,
    context: LayoutContext,
    grid_template_rows: Vec<GridTemplateComponent<String>>,
    grid_template_columns: Vec<GridTemplateComponent<String>>,
    grid_auto_rows: Vec<TrackSizingFunction>,
    grid_auto_columns: Vec<TrackSizingFunction>,
}

impl CoreNode {
    pub(super) fn from_node(node: &Node, context: LayoutContext) -> Self {
        Self {
            node: node.clone(),
            context,
            grid_template_rows: node
                .grid_template_rows
                .iter()
                .map(|track| track.clone_into_repeated_taffy_track(&context))
                .collect(),
            grid_template_columns: node
                .grid_template_columns
                .iter()
                .map(|track| track.clone_into_repeated_taffy_track(&context))
                .collect(),
            grid_auto_rows: node
                .grid_auto_rows
                .iter()
                .map(|track| track.into_taffy_track(&context))
                .collect(),
            grid_auto_columns: node
                .grid_auto_columns
                .iter()
                .map(|track| track.into_taffy_track(&context))
                .collect(),
        }
    }

    pub(super) fn viewport() -> Self {
        Self::from_node(
            &Node {
                display: UiDisplay::Grid,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: UiAlignItems::Start,
                justify_items: UiJustifyItems::Start,
                ..Default::default()
            },
            LayoutContext::DEFAULT,
        )
    }

    pub(super) fn to_taffy_style(&self) -> Style {
        convert::from_node(&self.node, &self.context)
    }

    pub(super) fn display(&self) -> Display {
        self.node.display.into()
    }
}

impl CoreStyle for CoreNode {
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
        self.node.box_sizing.into()
    }

    #[inline(always)]
    fn direction(&self) -> Direction {
        self.node.direction.into()
    }

    #[inline(always)]
    fn overflow(&self) -> Point<Overflow> {
        Point {
            x: self.node.overflow.x.into(),
            y: self.node.overflow.y.into(),
        }
    }

    #[inline(always)]
    fn scrollbar_width(&self) -> f32 {
        self.node.scrollbar_width * self.context.scale_factor
    }

    #[inline(always)]
    fn position(&self) -> Position {
        self.node.position_type.into()
    }

    #[inline(always)]
    fn inset(&self) -> Rect<LengthPercentageAuto> {
        Rect {
            left: self.node.left.into_length_percentage_auto(&self.context),
            right: self.node.right.into_length_percentage_auto(&self.context),
            top: self.node.top.into_length_percentage_auto(&self.context),
            bottom: self.node.bottom.into_length_percentage_auto(&self.context),
        }
    }

    #[inline(always)]
    fn size(&self) -> Size<Dimension> {
        Size {
            width: self.node.width.into_dimension(&self.context),
            height: self.node.height.into_dimension(&self.context),
        }
    }

    #[inline(always)]
    fn min_size(&self) -> Size<Dimension> {
        Size {
            width: self.node.min_width.into_dimension(&self.context),
            height: self.node.min_height.into_dimension(&self.context),
        }
    }

    #[inline(always)]
    fn max_size(&self) -> Size<Dimension> {
        Size {
            width: self.node.max_width.into_dimension(&self.context),
            height: self.node.max_height.into_dimension(&self.context),
        }
    }

    #[inline(always)]
    fn aspect_ratio(&self) -> Option<f32> {
        self.node.aspect_ratio
    }

    #[inline(always)]
    fn margin(&self) -> Rect<LengthPercentageAuto> {
        self.node
            .margin
            .map_to_taffy_rect(|margin| margin.into_length_percentage_auto(&self.context))
    }

    #[inline(always)]
    fn padding(&self) -> Rect<LengthPercentage> {
        self.node
            .padding
            .map_to_taffy_rect(|padding| padding.into_length_percentage(&self.context))
    }

    #[inline(always)]
    fn border(&self) -> Rect<LengthPercentage> {
        self.node
            .border
            .map_to_taffy_rect(|border| border.into_length_percentage(&self.context))
    }
}

impl BlockContainerStyle for CoreNode {
    #[inline(always)]
    fn text_align(&self) -> TextAlign {
        TextAlign::Auto
    }
}

impl BlockItemStyle for CoreNode {}

impl FlexboxContainerStyle for CoreNode {
    #[inline(always)]
    fn flex_direction(&self) -> FlexDirection {
        self.node.flex_direction.into()
    }

    #[inline(always)]
    fn flex_wrap(&self) -> FlexWrap {
        self.node.flex_wrap.into()
    }

    #[inline(always)]
    fn gap(&self) -> Size<LengthPercentage> {
        Size {
            width: self.node.column_gap.into_length_percentage(&self.context),
            height: self.node.row_gap.into_length_percentage(&self.context),
        }
    }

    #[inline(always)]
    fn align_content(&self) -> Option<AlignContent> {
        self.node.align_content.into()
    }

    #[inline(always)]
    fn align_items(&self) -> Option<AlignItems> {
        self.node.align_items.into()
    }

    #[inline(always)]
    fn justify_content(&self) -> Option<JustifyContent> {
        self.node.justify_content.into()
    }
}

impl FlexboxItemStyle for CoreNode {
    #[inline(always)]
    fn flex_basis(&self) -> Dimension {
        self.node.flex_basis.into_dimension(&self.context)
    }

    #[inline(always)]
    fn flex_grow(&self) -> f32 {
        self.node.flex_grow
    }

    #[inline(always)]
    fn flex_shrink(&self) -> f32 {
        self.node.flex_shrink
    }

    #[inline(always)]
    fn align_self(&self) -> Option<AlignSelf> {
        self.node.align_self.into()
    }
}

impl GridContainerStyle for CoreNode {
    type Repetition<'a>
        = &'a GridTemplateRepetition<String>
    where
        Self: 'a;

    type TemplateTrackList<'a>
        = core::iter::Map<
        slice::Iter<'a, GridTemplateComponent<String>>,
        fn(
            &'a GridTemplateComponent<String>,
        ) -> GenericGridTemplateComponent<String, &'a GridTemplateRepetition<String>>,
    >
    where
        Self: 'a;

    type AutoTrackList<'a>
        = core::iter::Copied<slice::Iter<'a, TrackSizingFunction>>
    where
        Self: 'a;

    type TemplateLineNames<'a>
        = core::iter::Map<slice::Iter<'a, Vec<String>>, fn(&Vec<String>) -> slice::Iter<'_, String>>
    where
        Self: 'a;

    type GridTemplateAreas<'a>
        = core::iter::Empty<GridTemplateArea<String>>
    where
        Self: 'a;

    #[inline(always)]
    fn grid_template_rows(&self) -> Option<Self::TemplateTrackList<'_>> {
        Some(self.grid_template_rows.iter().map(as_component_ref))
    }

    #[inline(always)]
    fn grid_template_columns(&self) -> Option<Self::TemplateTrackList<'_>> {
        Some(self.grid_template_columns.iter().map(as_component_ref))
    }

    #[inline(always)]
    fn grid_auto_rows(&self) -> Self::AutoTrackList<'_> {
        self.grid_auto_rows.iter().copied()
    }

    #[inline(always)]
    fn grid_auto_columns(&self) -> Self::AutoTrackList<'_> {
        self.grid_auto_columns.iter().copied()
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
        self.node.grid_auto_flow.into()
    }

    #[inline(always)]
    fn gap(&self) -> Size<LengthPercentage> {
        Size {
            width: self.node.column_gap.into_length_percentage(&self.context),
            height: self.node.row_gap.into_length_percentage(&self.context),
        }
    }

    #[inline(always)]
    fn align_content(&self) -> Option<AlignContent> {
        self.node.align_content.into()
    }

    #[inline(always)]
    fn justify_content(&self) -> Option<JustifyContent> {
        self.node.justify_content.into()
    }

    #[inline(always)]
    fn align_items(&self) -> Option<AlignItems> {
        self.node.align_items.into()
    }

    #[inline(always)]
    fn justify_items(&self) -> Option<AlignItems> {
        self.node.justify_items.into()
    }
}

impl GridItemStyle for CoreNode {
    #[inline(always)]
    fn grid_row(&self) -> Line<TaffyGridPlacement<String>> {
        self.node.grid_row.into()
    }

    #[inline(always)]
    fn grid_column(&self) -> Line<TaffyGridPlacement<String>> {
        self.node.grid_column.into()
    }

    #[inline(always)]
    fn align_self(&self) -> Option<AlignSelf> {
        self.node.align_self.into()
    }

    #[inline(always)]
    fn justify_self(&self) -> Option<AlignSelf> {
        self.node.justify_self.into()
    }
}
