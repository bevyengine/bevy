use taffy::style_helpers;

use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, GridAutoFlow,
    GridPlacement, GridTrack, GridTrackRepetition, JustifyContent, JustifyItems, JustifySelf,
    MaxTrackSizingFunction, MinTrackSizingFunction, OverflowAxis, PositionType, RepeatedGridTrack,
    Style, UiRect, Val,
};

use super::LayoutContext;

impl Val {
    fn into_length_percentage_auto(
        self,
        context: &LayoutContext,
    ) -> taffy::style::LengthPercentageAuto {
        match self {
            Val::Auto => taffy::style::LengthPercentageAuto::Auto,
            Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.),
            Val::Px(value) => {
                taffy::style::LengthPercentageAuto::Length(context.scale_factor * value)
            }
            Val::VMin(value) => {
                taffy::style::LengthPercentageAuto::Length(context.min_size * value / 100.)
            }
            Val::VMax(value) => {
                taffy::style::LengthPercentageAuto::Length(context.max_size * value / 100.)
            }
            Val::Vw(value) => {
                taffy::style::LengthPercentageAuto::Length(context.physical_size.x * value / 100.)
            }
            Val::Vh(value) => {
                taffy::style::LengthPercentageAuto::Length(context.physical_size.y * value / 100.)
            }
        }
    }

    fn into_length_percentage(self, context: &LayoutContext) -> taffy::style::LengthPercentage {
        match self.into_length_percentage_auto(context) {
            taffy::style::LengthPercentageAuto::Auto => taffy::style::LengthPercentage::Length(0.0),
            taffy::style::LengthPercentageAuto::Percent(value) => {
                taffy::style::LengthPercentage::Percent(value)
            }
            taffy::style::LengthPercentageAuto::Length(value) => {
                taffy::style::LengthPercentage::Length(value)
            }
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

pub fn from_style(
    context: &LayoutContext,
    style: &Style,
    ignore_border: bool,
) -> taffy::style::Style {
    taffy::style::Style {
        display: style.display.into(),
        overflow: taffy::Point {
            x: style.overflow.x.into(),
            y: style.overflow.y.into(),
        },
        scrollbar_width: 0.0,
        position: style.position_type.into(),
        flex_direction: style.flex_direction.into(),
        flex_wrap: style.flex_wrap.into(),
        align_items: style.align_items.into(),
        justify_items: style.justify_items.into(),
        align_self: style.align_self.into(),
        justify_self: style.justify_self.into(),
        align_content: style.align_content.into(),
        justify_content: style.justify_content.into(),
        inset: taffy::Rect {
            left: style.left.into_length_percentage_auto(context),
            right: style.right.into_length_percentage_auto(context),
            top: style.top.into_length_percentage_auto(context),
            bottom: style.bottom.into_length_percentage_auto(context),
        },
        margin: style
            .margin
            .map_to_taffy_rect(|m| m.into_length_percentage_auto(context)),
        padding: style
            .padding
            .map_to_taffy_rect(|m| m.into_length_percentage(context)),
        // Ignore border for leaf nodes as it isn't implemented in the rendering engine.
        // TODO: Implement rendering of border for leaf nodes
        border: if ignore_border {
            taffy::Rect::zero()
        } else {
            style
                .border
                .map_to_taffy_rect(|m| m.into_length_percentage(context))
        },
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis.into_dimension(context),
        size: taffy::Size {
            width: style.width.into_dimension(context),
            height: style.height.into_dimension(context),
        },
        min_size: taffy::Size {
            width: style.min_width.into_dimension(context),
            height: style.min_height.into_dimension(context),
        },
        max_size: taffy::Size {
            width: style.max_width.into_dimension(context),
            height: style.max_height.into_dimension(context),
        },
        aspect_ratio: style.aspect_ratio,
        gap: taffy::Size {
            width: style.column_gap.into_length_percentage(context),
            height: style.row_gap.into_length_percentage(context),
        },
        grid_auto_flow: style.grid_auto_flow.into(),
        grid_template_rows: style
            .grid_template_rows
            .iter()
            .map(|track| track.clone_into_repeated_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_template_columns: style
            .grid_template_columns
            .iter()
            .map(|track| track.clone_into_repeated_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_auto_rows: style
            .grid_auto_rows
            .iter()
            .map(|track| track.into_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_auto_columns: style
            .grid_auto_columns
            .iter()
            .map(|track| track.into_taffy_track(context))
            .collect::<Vec<_>>(),
        grid_row: style.grid_row.into(),
        grid_column: style.grid_column.into(),
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

impl From<OverflowAxis> for taffy::style::Overflow {
    fn from(value: OverflowAxis) -> Self {
        match value {
            OverflowAxis::Visible => taffy::style::Overflow::Visible,
            OverflowAxis::Clip => taffy::style::Overflow::Clip,
            OverflowAxis::Hidden => taffy::style::Overflow::Hidden,
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

impl From<GridPlacement> for taffy::geometry::Line<taffy::style::GridPlacement> {
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
            MinTrackSizingFunction::Px(val) => taffy::style::MinTrackSizingFunction::Fixed(
                Val::Px(val).into_length_percentage(context),
            ),
            MinTrackSizingFunction::Percent(val) => taffy::style::MinTrackSizingFunction::Fixed(
                Val::Percent(val).into_length_percentage(context),
            ),
            MinTrackSizingFunction::Auto => taffy::style::MinTrackSizingFunction::Auto,
            MinTrackSizingFunction::MinContent => taffy::style::MinTrackSizingFunction::MinContent,
            MinTrackSizingFunction::MaxContent => taffy::style::MinTrackSizingFunction::MaxContent,
            MinTrackSizingFunction::VMin(val) => taffy::style::MinTrackSizingFunction::Fixed(
                Val::VMin(val).into_length_percentage(context),
            ),
            MinTrackSizingFunction::VMax(val) => taffy::style::MinTrackSizingFunction::Fixed(
                Val::VMax(val).into_length_percentage(context),
            ),
            MinTrackSizingFunction::Vh(val) => taffy::style::MinTrackSizingFunction::Fixed(
                Val::Vh(val).into_length_percentage(context),
            ),
            MinTrackSizingFunction::Vw(val) => taffy::style::MinTrackSizingFunction::Fixed(
                Val::Vw(val).into_length_percentage(context),
            ),
        }
    }
}

impl MaxTrackSizingFunction {
    fn into_taffy(self, context: &LayoutContext) -> taffy::style::MaxTrackSizingFunction {
        match self {
            MaxTrackSizingFunction::Px(val) => taffy::style::MaxTrackSizingFunction::Fixed(
                Val::Px(val).into_length_percentage(context),
            ),
            MaxTrackSizingFunction::Percent(val) => taffy::style::MaxTrackSizingFunction::Fixed(
                Val::Percent(val).into_length_percentage(context),
            ),
            MaxTrackSizingFunction::Auto => taffy::style::MaxTrackSizingFunction::Auto,
            MaxTrackSizingFunction::MinContent => taffy::style::MaxTrackSizingFunction::MinContent,
            MaxTrackSizingFunction::MaxContent => taffy::style::MaxTrackSizingFunction::MaxContent,
            MaxTrackSizingFunction::FitContentPx(val) => {
                taffy::style::MaxTrackSizingFunction::FitContent(
                    Val::Px(val).into_length_percentage(context),
                )
            }
            MaxTrackSizingFunction::FitContentPercent(val) => {
                taffy::style::MaxTrackSizingFunction::FitContent(
                    Val::Percent(val).into_length_percentage(context),
                )
            }
            MaxTrackSizingFunction::Fraction(fraction) => {
                taffy::style::MaxTrackSizingFunction::Fraction(fraction)
            }
            MaxTrackSizingFunction::VMin(val) => taffy::style::MaxTrackSizingFunction::Fixed(
                Val::VMin(val).into_length_percentage(context),
            ),
            MaxTrackSizingFunction::VMax(val) => taffy::style::MaxTrackSizingFunction::Fixed(
                Val::VMax(val).into_length_percentage(context),
            ),
            MaxTrackSizingFunction::Vh(val) => taffy::style::MaxTrackSizingFunction::Fixed(
                Val::Vh(val).into_length_percentage(context),
            ),
            MaxTrackSizingFunction::Vw(val) => taffy::style::MaxTrackSizingFunction::Fixed(
                Val::Vw(val).into_length_percentage(context),
            ),
        }
    }
}

impl GridTrack {
    fn into_taffy_track(
        self,
        context: &LayoutContext,
    ) -> taffy::style::NonRepeatedTrackSizingFunction {
        let min = self.min_sizing_function.into_taffy(context);
        let max = self.max_sizing_function.into_taffy(context);
        style_helpers::minmax(min, max)
    }
}

impl RepeatedGridTrack {
    fn clone_into_repeated_taffy_track(
        &self,
        context: &LayoutContext,
    ) -> taffy::style::TrackSizingFunction {
        if self.tracks.len() == 1 && self.repetition == GridTrackRepetition::Count(1) {
            let min = self.tracks[0].min_sizing_function.into_taffy(context);
            let max = self.tracks[0].max_sizing_function.into_taffy(context);
            let taffy_track = style_helpers::minmax(min, max);
            taffy::style::TrackSizingFunction::Single(taffy_track)
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
                    style_helpers::repeat(taffy::style::GridTrackRepetition::AutoFit, taffy_tracks)
                }
                GridTrackRepetition::AutoFill => {
                    style_helpers::repeat(taffy::style::GridTrackRepetition::AutoFill, taffy_tracks)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_from() {
        use sh::TaffyZero;
        use taffy::style_helpers as sh;

        let bevy_style = Style {
            display: Display::Flex,
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::Percent(50.),
            top: Val::Px(12.),
            bottom: Val::Auto,
            direction: crate::Direction::Inherit,
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
        let viewport_values = LayoutContext::new(1.0, bevy_math::Vec2::new(800., 600.));
        let taffy_style = from_style(&viewport_values, &bevy_style, false);
        assert_eq!(taffy_style.display, taffy::style::Display::Flex);
        assert_eq!(taffy_style.position, taffy::style::Position::Absolute);
        assert_eq!(
            taffy_style.inset.left,
            taffy::style::LengthPercentageAuto::ZERO
        );
        assert_eq!(
            taffy_style.inset.right,
            taffy::style::LengthPercentageAuto::Percent(0.5)
        );
        assert_eq!(
            taffy_style.inset.top,
            taffy::style::LengthPercentageAuto::Length(12.)
        );
        assert_eq!(
            taffy_style.inset.bottom,
            taffy::style::LengthPercentageAuto::Auto
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
            taffy::style::LengthPercentageAuto::Length(10.)
        );
        assert_eq!(
            taffy_style.margin.top,
            taffy::style::LengthPercentageAuto::Percent(0.15)
        );
        assert_eq!(
            taffy_style.margin.bottom,
            taffy::style::LengthPercentageAuto::Auto
        );
        assert_eq!(
            taffy_style.padding.left,
            taffy::style::LengthPercentage::Percent(0.13)
        );
        assert_eq!(
            taffy_style.padding.right,
            taffy::style::LengthPercentage::Length(21.)
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
            taffy::style::LengthPercentage::Length(14.)
        );
        assert_eq!(
            taffy_style.border.right,
            taffy::style::LengthPercentage::ZERO
        );
        assert_eq!(taffy_style.border.top, taffy::style::LengthPercentage::ZERO);
        assert_eq!(
            taffy_style.border.bottom,
            taffy::style::LengthPercentage::Percent(0.31)
        );
        assert_eq!(taffy_style.flex_grow, 1.);
        assert_eq!(taffy_style.flex_shrink, 0.);
        assert_eq!(taffy_style.flex_basis, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.size.width, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.size.height, taffy::style::Dimension::Auto);
        assert_eq!(taffy_style.min_size.width, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.min_size.height, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.max_size.width, taffy::style::Dimension::Auto);
        assert_eq!(taffy_style.max_size.height, taffy::style::Dimension::ZERO);
        assert_eq!(taffy_style.aspect_ratio, None);
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
                sh::fit_content(taffy::style::LengthPercentage::Length(10.0)),
                sh::fit_content(taffy::style::LengthPercentage::Percent(0.25)),
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
        let context = LayoutContext::new(2.0, bevy_math::Vec2::new(800., 600.));
        let cases = [
            (Val::Auto, LengthPercentage::Length(0.)),
            (Val::Percent(1.), LengthPercentage::Percent(0.01)),
            (Val::Px(1.), LengthPercentage::Length(2.)),
            (Val::Vw(1.), LengthPercentage::Length(8.)),
            (Val::Vh(1.), LengthPercentage::Length(6.)),
            (Val::VMin(2.), LengthPercentage::Length(12.)),
            (Val::VMax(2.), LengthPercentage::Length(16.)),
        ];
        for (val, length) in cases {
            assert!(match (val.into_length_percentage(&context), length) {
                (LengthPercentage::Length(a), LengthPercentage::Length(b))
                | (LengthPercentage::Percent(a), LengthPercentage::Percent(b)) =>
                    (a - b).abs() < 0.0001,
                _ => false,
            });
        }
    }
}
