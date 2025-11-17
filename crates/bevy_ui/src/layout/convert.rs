use taffy::style_helpers;

use crate::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Display, FlexDirection, FlexWrap, GridAutoFlow,
    GridPlacement, GridTrack, GridTrackRepetition, JustifyContent, JustifyItems, JustifySelf,
    MaxTrackSizingFunction, MinTrackSizingFunction, Node, OverflowAxis, PositionType,
    RepeatedGridTrack, UiRect, Val,
};

use super::LayoutContext;

impl Val {
    fn into_length_percentage_auto(
        self,
        context: &LayoutContext,
    ) -> taffy::style::LengthPercentageAuto {
        match self {
            Val::Auto => style_helpers::auto(),
            Val::Percent(value) => style_helpers::percent(value / 100.),
            Val::Px(value) => style_helpers::length(context.scale_factor * value),
            Val::VMin(value) => {
                style_helpers::length(context.physical_size.min_element() * value / 100.)
            }
            Val::VMax(value) => {
                style_helpers::length(context.physical_size.max_element() * value / 100.)
            }
            Val::Vw(value) => style_helpers::length(context.physical_size.x * value / 100.),
            Val::Vh(value) => style_helpers::length(context.physical_size.y * value / 100.),
        }
    }

    fn into_length_percentage(self, context: &LayoutContext) -> taffy::style::LengthPercentage {
        match self {
            Val::Auto => style_helpers::length(0.),
            Val::Percent(value) => style_helpers::percent(value / 100.),
            Val::Px(value) => style_helpers::length(context.scale_factor * value),
            Val::VMin(value) => {
                style_helpers::length(context.physical_size.min_element() * value / 100.)
            }
            Val::VMax(value) => {
                style_helpers::length(context.physical_size.max_element() * value / 100.)
            }
            Val::Vw(value) => style_helpers::length(context.physical_size.x * value / 100.),
            Val::Vh(value) => style_helpers::length(context.physical_size.y * value / 100.),
        }
    }

    fn into_dimension(self, context: &LayoutContext) -> taffy::style::Dimension {
        self.into_length_percentage_auto(context).into()
    }
}

impl UiRect {
    fn map_to_taffy_rect<T>(self, map_fn: impl Fn(Val) -> T) -> taffy::geometry::Rect<T> {
        taffy::geometry::Rect {
            left: map_fn(self.left),
            right: map_fn(self.right),
            top: map_fn(self.top),
            bottom: map_fn(self.bottom),
        }
    }
}

pub fn from_node(node: &Node, context: &LayoutContext, ignore_border: bool) -> taffy::style::Style {
    taffy::style::Style {
        display: node.display.into(),
        box_sizing: node.box_sizing.into(),
        item_is_table: false,
        text_align: taffy::TextAlign::Auto,
        overflow: taffy::Point {
            x: node.overflow.x.into(),
            y: node.overflow.y.into(),
        },
        scrollbar_width: node.scrollbar_width * context.scale_factor,
        position: node.position_type.into(),
        flex_direction: node.flex_direction.into(),
        flex_wrap: node.flex_wrap.into(),
        align_items: node.align_items.into(),
        justify_items: node.justify_items.into(),
        align_self: node.align_self.into(),
        justify_self: node.justify_self.into(),
        align_content: node.align_content.into(),
        justify_content: node.justify_content.into(),
        inset: taffy::Rect {
            left: node.left.into_length_percentage_auto(context),
            right: node.right.into_length_percentage_auto(context),
            top: node.top.into_length_percentage_auto(context),
            bottom: node.bottom.into_length_percentage_auto(context),
        },
        margin: node
            .margin
            .map_to_taffy_rect(|m| m.into_length_percentage_auto(context)),
        padding: node
            .padding
            .map_to_taffy_rect(|m| m.into_length_percentage(context)),
        // Ignore border for leaf nodes as it isn't implemented in the rendering engine.
        // TODO: Implement rendering of border for leaf nodes
        border: if ignore_border {
            taffy::Rect::zero()
        } else {
            node.border
                .map_to_taffy_rect(|m| m.into_length_percentage(context))
        },
        flex_grow: node.flex_grow,
        flex_shrink: node.flex_shrink,
        flex_basis: node.flex_basis.into_dimension(context),
        size: taffy::Size {
            width: node.width.into_dimension(context),
            height: node.height.into_dimension(context),
        },
        min_size: taffy::Size {
            width: node.min_width.into_dimension(context),
            height: node.min_height.into_dimension(context),
        },
        max_size: taffy::Size {
            width: node.max_width.into_dimension(context),
            height: node.max_height.into_dimension(context),
        },
        aspect_ratio: node.aspect_ratio,
        gap: taffy::Size {
            width: node.column_gap.into_length_percentage(context),
            height: node.row_gap.into_length_percentage(context),
        },
        grid_auto_flow: node.grid_auto_flow.into(),
        grid_template_rows: node
            .grid_template_rows
            .iter()
            .map(|track| track.clone_into_repeated_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_template_columns: node
            .grid_template_columns
            .iter()
            .map(|track| track.clone_into_repeated_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_auto_rows: node
            .grid_auto_rows
            .iter()
            .map(|track| track.into_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_auto_columns: node
            .grid_auto_columns
            .iter()
            .map(|track| track.into_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_row: node.grid_row.into(),
        grid_column: node.grid_column.into(),
        ..Default::default()
    }
}

impl From<AlignItems> for Option<taffy::style::AlignItems> {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::Default => None,
            AlignItems::Start => taffy::style::AlignItems::Start.into(),
            AlignItems::End => taffy::style::AlignItems::End.into(),
            AlignItems::FlexStart => taffy::style::AlignItems::FlexStart.into(),
            AlignItems::FlexEnd => taffy::style::AlignItems::FlexEnd.into(),
            AlignItems::Center => taffy::style::AlignItems::Center.into(),
            AlignItems::Baseline => taffy::style::AlignItems::Baseline.into(),
            AlignItems::Stretch => taffy::style::AlignItems::Stretch.into(),
        }
    }
}

impl From<JustifyItems> for Option<taffy::style::JustifyItems> {
    fn from(value: JustifyItems) -> Self {
        match value {
            JustifyItems::Default => None,
            JustifyItems::Start => taffy::style::JustifyItems::Start.into(),
            JustifyItems::End => taffy::style::JustifyItems::End.into(),
            JustifyItems::Center => taffy::style::JustifyItems::Center.into(),
            JustifyItems::Baseline => taffy::style::JustifyItems::Baseline.into(),
            JustifyItems::Stretch => taffy::style::JustifyItems::Stretch.into(),
        }
    }
}

impl From<AlignSelf> for Option<taffy::style::AlignSelf> {
    fn from(value: AlignSelf) -> Self {
        match value {
            AlignSelf::Auto => None,
            AlignSelf::Start => taffy::style::AlignSelf::Start.into(),
            AlignSelf::End => taffy::style::AlignSelf::End.into(),
            AlignSelf::FlexStart => taffy::style::AlignSelf::FlexStart.into(),
            AlignSelf::FlexEnd => taffy::style::AlignSelf::FlexEnd.into(),
            AlignSelf::Center => taffy::style::AlignSelf::Center.into(),
            AlignSelf::Baseline => taffy::style::AlignSelf::Baseline.into(),
            AlignSelf::Stretch => taffy::style::AlignSelf::Stretch.into(),
        }
    }
}

impl From<JustifySelf> for Option<taffy::style::JustifySelf> {
    fn from(value: JustifySelf) -> Self {
        match value {
            JustifySelf::Auto => None,
            JustifySelf::Start => taffy::style::JustifySelf::Start.into(),
            JustifySelf::End => taffy::style::JustifySelf::End.into(),
            JustifySelf::Center => taffy::style::JustifySelf::Center.into(),
            JustifySelf::Baseline => taffy::style::JustifySelf::Baseline.into(),
            JustifySelf::Stretch => taffy::style::JustifySelf::Stretch.into(),
        }
    }
}

impl From<AlignContent> for Option<taffy::style::AlignContent> {
    fn from(value: AlignContent) -> Self {
        match value {
            AlignContent::Default => None,
            AlignContent::Start => taffy::style::AlignContent::Start.into(),
            AlignContent::End => taffy::style::AlignContent::End.into(),
            AlignContent::FlexStart => taffy::style::AlignContent::FlexStart.into(),
            AlignContent::FlexEnd => taffy::style::AlignContent::FlexEnd.into(),
            AlignContent::Center => taffy::style::AlignContent::Center.into(),
            AlignContent::Stretch => taffy::style::AlignContent::Stretch.into(),
            AlignContent::SpaceBetween => taffy::style::AlignContent::SpaceBetween.into(),
            AlignContent::SpaceAround => taffy::style::AlignContent::SpaceAround.into(),
            AlignContent::SpaceEvenly => taffy::style::AlignContent::SpaceEvenly.into(),
        }
    }
}

impl From<JustifyContent> for Option<taffy::style::JustifyContent> {
    fn from(value: JustifyContent) -> Self {
        match value {
            JustifyContent::Default => None,
            JustifyContent::Start => taffy::style::JustifyContent::Start.into(),
            JustifyContent::End => taffy::style::JustifyContent::End.into(),
            JustifyContent::FlexStart => taffy::style::JustifyContent::FlexStart.into(),
            JustifyContent::FlexEnd => taffy::style::JustifyContent::FlexEnd.into(),
            JustifyContent::Center => taffy::style::JustifyContent::Center.into(),
            JustifyContent::Stretch => taffy::style::JustifyContent::Stretch.into(),
            JustifyContent::SpaceBetween => taffy::style::JustifyContent::SpaceBetween.into(),
            JustifyContent::SpaceAround => taffy::style::JustifyContent::SpaceAround.into(),
            JustifyContent::SpaceEvenly => taffy::style::JustifyContent::SpaceEvenly.into(),
        }
    }
}

impl From<Display> for taffy::style::Display {
    fn from(value: Display) -> Self {
        match value {
            Display::Flex => taffy::style::Display::Flex,
            Display::Grid => taffy::style::Display::Grid,
            Display::Block => taffy::style::Display::Block,
            Display::None => taffy::style::Display::None,
        }
    }
}

impl From<BoxSizing> for taffy::style::BoxSizing {
    fn from(value: BoxSizing) -> Self {
        match value {
            BoxSizing::BorderBox => taffy::style::BoxSizing::BorderBox,
            BoxSizing::ContentBox => taffy::style::BoxSizing::ContentBox,
        }
    }
}

impl From<OverflowAxis> for taffy::style::Overflow {
    fn from(value: OverflowAxis) -> Self {
        match value {
            OverflowAxis::Visible => taffy::style::Overflow::Visible,
            OverflowAxis::Clip => taffy::style::Overflow::Clip,
            OverflowAxis::Hidden => taffy::style::Overflow::Hidden,
            OverflowAxis::Scroll => taffy::style::Overflow::Scroll,
        }
    }
}

impl From<FlexDirection> for taffy::style::FlexDirection {
    fn from(value: FlexDirection) -> Self {
        match value {
            FlexDirection::Row => taffy::style::FlexDirection::Row,
            FlexDirection::Column => taffy::style::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::style::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::style::FlexDirection::ColumnReverse,
        }
    }
}

impl From<PositionType> for taffy::style::Position {
    fn from(value: PositionType) -> Self {
        match value {
            PositionType::Relative => taffy::style::Position::Relative,
            PositionType::Absolute => taffy::style::Position::Absolute,
        }
    }
}

impl From<FlexWrap> for taffy::style::FlexWrap {
    fn from(value: FlexWrap) -> Self {
        match value {
            FlexWrap::NoWrap => taffy::style::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::style::FlexWrap::Wrap,
            FlexWrap::WrapReverse => taffy::style::FlexWrap::WrapReverse,
        }
    }
}

impl From<GridAutoFlow> for taffy::style::GridAutoFlow {
    fn from(value: GridAutoFlow) -> Self {
        match value {
            GridAutoFlow::Row => taffy::style::GridAutoFlow::Row,
            GridAutoFlow::RowDense => taffy::style::GridAutoFlow::RowDense,
            GridAutoFlow::Column => taffy::style::GridAutoFlow::Column,
            GridAutoFlow::ColumnDense => taffy::style::GridAutoFlow::ColumnDense,
        }
    }
}

impl From<GridPlacement> for taffy::geometry::Line<taffy::style::GridPlacement<String>> {
    fn from(value: GridPlacement) -> Self {
        let span = value.get_span().unwrap_or(1);
        match (value.get_start(), value.get_end()) {
            (Some(start), Some(end)) => taffy::geometry::Line {
                start: style_helpers::line(start),
                end: style_helpers::line(end),
            },
            (Some(start), None) => taffy::geometry::Line {
                start: style_helpers::line(start),
                end: style_helpers::span(span),
            },
            (None, Some(end)) => taffy::geometry::Line {
                start: style_helpers::span(span),
                end: style_helpers::line(end),
            },
            (None, None) => style_helpers::span(span),
        }
    }
}

impl MinTrackSizingFunction {
    fn into_taffy(self, context: &LayoutContext) -> taffy::style::MinTrackSizingFunction {
        match self {
            MinTrackSizingFunction::Px(val) => Val::Px(val).into_length_percentage(context).into(),
            MinTrackSizingFunction::Percent(val) => {
                Val::Percent(val).into_length_percentage(context).into()
            }
            MinTrackSizingFunction::Auto => taffy::style::MinTrackSizingFunction::auto(),
            MinTrackSizingFunction::MinContent => {
                taffy::style::MinTrackSizingFunction::min_content()
            }
            MinTrackSizingFunction::MaxContent => {
                taffy::style::MinTrackSizingFunction::max_content()
            }
            MinTrackSizingFunction::VMin(val) => {
                Val::VMin(val).into_length_percentage(context).into()
            }
            MinTrackSizingFunction::VMax(val) => {
                Val::VMax(val).into_length_percentage(context).into()
            }
            MinTrackSizingFunction::Vh(val) => Val::Vh(val).into_length_percentage(context).into(),
            MinTrackSizingFunction::Vw(val) => Val::Vw(val).into_length_percentage(context).into(),
        }
    }
}

impl MaxTrackSizingFunction {
    fn into_taffy(self, context: &LayoutContext) -> taffy::style::MaxTrackSizingFunction {
        match self {
            MaxTrackSizingFunction::Px(val) => Val::Px(val).into_length_percentage(context).into(),
            MaxTrackSizingFunction::Percent(val) => {
                Val::Percent(val).into_length_percentage(context).into()
            }
            MaxTrackSizingFunction::Auto => taffy::style::MaxTrackSizingFunction::auto(),
            MaxTrackSizingFunction::MinContent => {
                taffy::style::MaxTrackSizingFunction::min_content()
            }
            MaxTrackSizingFunction::MaxContent => {
                taffy::style::MaxTrackSizingFunction::max_content()
            }
            MaxTrackSizingFunction::FitContentPx(val) => {
                taffy::style::MaxTrackSizingFunction::fit_content_px(
                    Val::Px(val)
                        .into_length_percentage(context)
                        .into_raw()
                        .value(),
                )
            }
            MaxTrackSizingFunction::FitContentPercent(val) => {
                taffy::style::MaxTrackSizingFunction::fit_content_percent(
                    Val::Percent(val)
                        .into_length_percentage(context)
                        .into_raw()
                        .value(),
                )
            }
            MaxTrackSizingFunction::Fraction(fraction) => {
                taffy::style::MaxTrackSizingFunction::fr(fraction)
            }
            MaxTrackSizingFunction::VMin(val) => {
                Val::VMin(val).into_length_percentage(context).into()
            }
            MaxTrackSizingFunction::VMax(val) => {
                Val::VMax(val).into_length_percentage(context).into()
            }
            MaxTrackSizingFunction::Vh(val) => Val::Vh(val).into_length_percentage(context).into(),
            MaxTrackSizingFunction::Vw(val) => Val::Vw(val).into_length_percentage(context).into(),
        }
    }
}

impl GridTrack {
    fn into_taffy_track(self, context: &LayoutContext) -> taffy::style::TrackSizingFunction {
        let min = self.min_sizing_function.into_taffy(context);
        let max = self.max_sizing_function.into_taffy(context);
        style_helpers::minmax(min, max)
    }
}

impl RepeatedGridTrack {
    fn clone_into_repeated_taffy_track(
        &self,
        context: &LayoutContext,
    ) -> taffy::style::GridTemplateComponent<String> {
        if self.tracks.len() == 1 && self.repetition == GridTrackRepetition::Count(1) {
            let min = self.tracks[0].min_sizing_function.into_taffy(context);
            let max = self.tracks[0].max_sizing_function.into_taffy(context);
            let taffy_track = style_helpers::minmax(min, max);
            taffy::GridTemplateComponent::Single(taffy_track)
        } else {
            let taffy_tracks: Vec<_> = self
                .tracks
                .iter()
                .map(|track| {
                    let min = track.min_sizing_function.into_taffy(context);
                    let max = track.max_sizing_function.into_taffy(context);
                    style_helpers::minmax(min, max)
                })
                .collect();

            match self.repetition {
                GridTrackRepetition::Count(count) => style_helpers::repeat(count, taffy_tracks),
                GridTrackRepetition::AutoFit => {
                    style_helpers::repeat(taffy::style::RepetitionCount::AutoFit, taffy_tracks)
                }
                GridTrackRepetition::AutoFill => {
                    style_helpers::repeat(taffy::style::RepetitionCount::AutoFill, taffy_tracks)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::Vec2;

    use crate::BorderRadius;

    use super::*;

    #[test]
    fn test_convert_from() {
        use sh::TaffyZero;
        use taffy::style_helpers as sh;

        let node = Node {
            display: Display::Flex,
            box_sizing: BoxSizing::ContentBox,
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::Percent(50.),
            top: Val::Px(12.),
            bottom: Val::Auto,
            flex_direction: FlexDirection::ColumnReverse,
            flex_wrap: FlexWrap::WrapReverse,
            align_items: AlignItems::Baseline,
            align_self: AlignSelf::Start,
            align_content: AlignContent::SpaceAround,
            justify_items: JustifyItems::Default,
            justify_self: JustifySelf::Center,
            justify_content: JustifyContent::SpaceEvenly,
            margin: UiRect {
                left: Val::ZERO,
                right: Val::Px(10.),
                top: Val::Percent(15.),
                bottom: Val::Auto,
            },
            padding: UiRect {
                left: Val::Percent(13.),
                right: Val::Px(21.),
                top: Val::Auto,
                bottom: Val::ZERO,
            },
            border: UiRect {
                left: Val::Px(14.),
                right: Val::ZERO,
                top: Val::Auto,
                bottom: Val::Percent(31.),
            },
            border_radius: BorderRadius::DEFAULT,
            flex_grow: 1.,
            flex_shrink: 0.,
            flex_basis: Val::ZERO,
            width: Val::ZERO,
            height: Val::Auto,
            min_width: Val::ZERO,
            min_height: Val::ZERO,
            max_width: Val::Auto,
            max_height: Val::ZERO,
            aspect_ratio: None,
            overflow: crate::Overflow::clip(),
            overflow_clip_margin: crate::OverflowClipMargin::default(),
            scrollbar_width: 7.,
            column_gap: Val::ZERO,
            row_gap: Val::ZERO,
            grid_auto_flow: GridAutoFlow::ColumnDense,
            grid_template_rows: vec![
                GridTrack::px(10.0),
                GridTrack::percent(50.0),
                GridTrack::fr(1.0),
            ],
            grid_template_columns: RepeatedGridTrack::px(5, 10.0),
            grid_auto_rows: vec![
                GridTrack::fit_content_px(10.0),
                GridTrack::fit_content_percent(25.0),
                GridTrack::flex(2.0),
            ],
            grid_auto_columns: vec![
                GridTrack::auto(),
                GridTrack::min_content(),
                GridTrack::max_content(),
            ],
            grid_column: GridPlacement::start(4),
            grid_row: GridPlacement::span(3),
        };
        let viewport_values = LayoutContext::new(1.0, Vec2::new(800., 600.));
        let taffy_style = from_node(&node, &viewport_values, false);
        assert_eq!(taffy_style.display, taffy::style::Display::Flex);
        assert_eq!(taffy_style.box_sizing, taffy::style::BoxSizing::ContentBox);
        assert_eq!(taffy_style.position, taffy::style::Position::Absolute);
        assert_eq!(
            taffy_style.inset.left,
            taffy::style::LengthPercentageAuto::ZERO
        );
        assert_eq!(
            taffy_style.inset.right,
            taffy::style::LengthPercentageAuto::percent(0.5)
        );
        assert_eq!(
            taffy_style.inset.top,
            taffy::style::LengthPercentageAuto::length(12.)
        );
        assert_eq!(
            taffy_style.inset.bottom,
            taffy::style::LengthPercentageAuto::auto()
        );
        assert_eq!(
            taffy_style.flex_direction,
            taffy::style::FlexDirection::ColumnReverse
        );
        assert_eq!(taffy_style.flex_wrap, taffy::style::FlexWrap::WrapReverse);
        assert_eq!(
            taffy_style.align_items,
            Some(taffy::style::AlignItems::Baseline)
        );
        assert_eq!(taffy_style.align_self, Some(taffy::style::AlignSelf::Start));
        assert_eq!(
            taffy_style.align_content,
            Some(taffy::style::AlignContent::SpaceAround)
        );
        assert_eq!(
            taffy_style.justify_content,
            Some(taffy::style::JustifyContent::SpaceEvenly)
        );
        assert_eq!(taffy_style.justify_items, None);
        assert_eq!(
            taffy_style.justify_self,
            Some(taffy::style::JustifySelf::Center)
        );
        assert_eq!(
            taffy_style.margin.left,
            taffy::style::LengthPercentageAuto::ZERO
        );
        assert_eq!(
            taffy_style.margin.right,
            taffy::style::LengthPercentageAuto::length(10.)
        );
        assert_eq!(
            taffy_style.margin.top,
            taffy::style::LengthPercentageAuto::percent(0.15)
        );
        assert_eq!(
            taffy_style.margin.bottom,
            taffy::style::LengthPercentageAuto::auto()
        );
        assert_eq!(
            taffy_style.padding.left,
            taffy::style::LengthPercentage::percent(0.13)
        );
        assert_eq!(
            taffy_style.padding.right,
            taffy::style::LengthPercentage::length(21.)
        );
        assert_eq!(
            taffy_style.padding.top,
            taffy::style::LengthPercentage::ZERO
        );
        assert_eq!(
            taffy_style.padding.bottom,
            taffy::style::LengthPercentage::ZERO
        );
        assert_eq!(
            taffy_style.border.left,
            taffy::style::LengthPercentage::length(14.)
        );
        assert_eq!(
            taffy_style.border.right,
            taffy::style::LengthPercentage::ZERO
        );
        assert_eq!(taffy_style.border.top, taffy::style::LengthPercentage::ZERO);
        assert_eq!(
            taffy_style.border.bottom,
            taffy::style::LengthPercentage::percent(0.31)
        );
        assert_eq!(taffy_style.flex_grow, 1.);
        assert_eq!(taffy_style.flex_shrink, 0.);
        assert_eq!(taffy_style.flex_basis, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.size.width, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.size.height, taffy::style::Dimension::auto());
        assert_eq!(taffy_style.min_size.width, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.min_size.height, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.max_size.width, taffy::style::Dimension::auto());
        assert_eq!(taffy_style.max_size.height, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.aspect_ratio, None);
        assert_eq!(taffy_style.scrollbar_width, 7.);
        assert_eq!(taffy_style.gap.width, taffy::style::LengthPercentage::ZERO);
        assert_eq!(taffy_style.gap.height, taffy::style::LengthPercentage::ZERO);
        assert_eq!(
            taffy_style.grid_auto_flow,
            taffy::style::GridAutoFlow::ColumnDense
        );
        assert_eq!(
            taffy_style.grid_template_rows,
            vec![sh::length(10.0), sh::percent(0.5), sh::fr(1.0)]
        );
        assert_eq!(
            taffy_style.grid_template_columns,
            vec![sh::repeat(5, vec![sh::length(10.0)])]
        );
        assert_eq!(
            taffy_style.grid_auto_rows,
            vec![
                sh::fit_content(taffy::style::LengthPercentage::length(10.0)),
                sh::fit_content(taffy::style::LengthPercentage::percent(0.25)),
                sh::minmax(sh::length(0.0), sh::fr(2.0)),
            ]
        );
        assert_eq!(
            taffy_style.grid_auto_columns,
            vec![sh::auto(), sh::min_content(), sh::max_content()]
        );
        assert_eq!(
            taffy_style.grid_column,
            taffy::geometry::Line {
                start: sh::line(4),
                end: sh::span(1)
            }
        );
        assert_eq!(taffy_style.grid_row, sh::span(3));
    }

    #[test]
    fn test_into_length_percentage() {
        use taffy::style::LengthPercentage;
        let context = LayoutContext::new(2.0, Vec2::new(800., 600.));
        let cases = [
            (Val::Auto, LengthPercentage::length(0.)),
            (Val::Percent(1.), LengthPercentage::percent(0.01)),
            (Val::Px(1.), LengthPercentage::length(2.)),
            (Val::Vw(1.), LengthPercentage::length(8.)),
            (Val::Vh(1.), LengthPercentage::length(6.)),
            (Val::VMin(2.), LengthPercentage::length(12.)),
            (Val::VMax(2.), LengthPercentage::length(16.)),
        ];
        for (val, length) in cases {
            assert!({
                let lhs = val.into_length_percentage(&context).into_raw().value();
                let rhs = length.into_raw().value();
                (lhs - rhs).abs() < 0.0001
            });
        }
    }
}
