use taffy::style_helpers;

use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, GridAutoFlow,
    GridPlacement, GridTrack, GridTrackRepetition, JustifyContent, JustifyItems, JustifySelf,
    MaxTrackSizingFunction, MinTrackSizingFunction, PositionType, RepeatedGridTrack, Size, Style,
    UiRect, Val,
};

impl Val {
    fn scaled(self, scale_factor: f64) -> Self {
        match self {
            Val::Auto => Val::Auto,
            Val::Percent(value) => Val::Percent(value),
            Val::Px(value) => Val::Px((scale_factor * value as f64) as f32),
        }
    }
    fn into_dimension(self, scale_factor: f64) -> taffy::style::Dimension {
        match self.scaled(scale_factor) {
            Val::Auto => taffy::style::Dimension::Auto,
            Val::Percent(value) => taffy::style::Dimension::Percent(value / 100.0),
            Val::Px(value) => taffy::style::Dimension::Points(value),
        }
    }
    fn into_length_percentage(self, scale_factor: f64) -> taffy::style::LengthPercentage {
        match self.scaled(scale_factor) {
            Val::Auto => taffy::style::LengthPercentage::Points(0.0),
            Val::Percent(value) => taffy::style::LengthPercentage::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentage::Points(value),
        }
    }
    fn into_length_percentage_auto(self, scale_factor: f64) -> taffy::style::LengthPercentageAuto {
        match self.scaled(scale_factor) {
            Val::Auto => taffy::style::LengthPercentageAuto::Auto,
            Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentageAuto::Points(value),
        }
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

impl Size {
    fn map_to_taffy_size<T>(self, map_fn: impl Fn(Val) -> T) -> taffy::geometry::Size<T> {
        taffy::geometry::Size {
            width: map_fn(self.width),
            height: map_fn(self.height),
        }
    }
}

pub fn from_style(scale_factor: f64, style: &Style) -> taffy::style::Style {
    taffy::style::Style {
        display: style.display.into(),
        position: style.position_type.into(),
        flex_direction: style.flex_direction.into(),
        flex_wrap: style.flex_wrap.into(),
        align_items: style.align_items.into(),
        justify_items: style.justify_items.into(),
        align_self: style.align_self.into(),
        justify_self: style.justify_self.into(),
        align_content: style.align_content.into(),
        justify_content: style.justify_content.into(),
        inset: taffy::prelude::Rect {
            left: style.left.into_length_percentage_auto(scale_factor),
            right: style.right.into_length_percentage_auto(scale_factor),
            top: style.top.into_length_percentage_auto(scale_factor),
            bottom: style.bottom.into_length_percentage_auto(scale_factor),
        },
        margin: style
            .margin
            .map_to_taffy_rect(|m| m.into_length_percentage_auto(scale_factor)),
        padding: style
            .padding
            .map_to_taffy_rect(|m| m.into_length_percentage(scale_factor)),
        border: style
            .border
            .map_to_taffy_rect(|m| m.into_length_percentage(scale_factor)),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis.into_dimension(scale_factor),
        size: style
            .size
            .map_to_taffy_size(|s| s.into_dimension(scale_factor)),
        min_size: style
            .min_size
            .map_to_taffy_size(|s| s.into_dimension(scale_factor)),
        max_size: style
            .max_size
            .map_to_taffy_size(|s| s.into_dimension(scale_factor)),
        aspect_ratio: style.aspect_ratio,
        gap: style
            .gap
            .map_to_taffy_size(|s| s.into_length_percentage(scale_factor)),
        grid_auto_flow: style.grid_auto_flow.into(),
        grid_template_rows: style
            .grid_template_rows
            .iter()
            .map(|track| track.into_repeated_taffy_track(scale_factor))
            .collect::<Vec<_>>(),
        grid_template_columns: style
            .grid_template_columns
            .iter()
            .map(|track| track.into_repeated_taffy_track(scale_factor))
            .collect::<Vec<_>>(),
        grid_auto_rows: style
            .grid_auto_rows
            .iter()
            .map(|track| track.into_taffy_track(scale_factor))
            .collect::<Vec<_>>(),
        grid_auto_columns: style
            .grid_auto_columns
            .iter()
            .map(|track| track.into_taffy_track(scale_factor))
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
            JustifyItems::FlexStart => taffy::style::JustifyItems::FlexStart.into(),
            JustifyItems::FlexEnd => taffy::style::JustifyItems::FlexEnd.into(),
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
            JustifySelf::FlexStart => taffy::style::JustifySelf::FlexStart.into(),
            JustifySelf::FlexEnd => taffy::style::JustifySelf::FlexEnd.into(),
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
            Display::None => taffy::style::Display::None,
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
        let start = match value.start {
            None => taffy::style::GridPlacement::Auto,
            Some(start) => style_helpers::line(start as i16),
        };
        let span = taffy::style::GridPlacement::Span(value.span);
        taffy::geometry::Line { start, end: span }
    }
}

impl MinTrackSizingFunction {
    fn into_taffy(self, scale_factor: f64) -> taffy::style::MinTrackSizingFunction {
        match self {
            MinTrackSizingFunction::Fixed(val) => taffy::style::MinTrackSizingFunction::Fixed(
                val.into_length_percentage(scale_factor),
            ),
            MinTrackSizingFunction::Auto => taffy::style::MinTrackSizingFunction::Auto,
            MinTrackSizingFunction::MinContent => taffy::style::MinTrackSizingFunction::MinContent,
            MinTrackSizingFunction::MaxContent => taffy::style::MinTrackSizingFunction::MaxContent,
        }
    }
}

impl MaxTrackSizingFunction {
    fn into_taffy(self, scale_factor: f64) -> taffy::style::MaxTrackSizingFunction {
        match self {
            MaxTrackSizingFunction::Fixed(val) => taffy::style::MaxTrackSizingFunction::Fixed(
                val.into_length_percentage(scale_factor),
            ),
            MaxTrackSizingFunction::Auto => taffy::style::MaxTrackSizingFunction::Auto,
            MaxTrackSizingFunction::MinContent => taffy::style::MaxTrackSizingFunction::MinContent,
            MaxTrackSizingFunction::MaxContent => taffy::style::MaxTrackSizingFunction::MaxContent,
            MaxTrackSizingFunction::FitContent(val) => {
                taffy::style::MaxTrackSizingFunction::FitContent(
                    val.into_length_percentage(scale_factor),
                )
            }
            MaxTrackSizingFunction::Fraction(fraction) => {
                taffy::style::MaxTrackSizingFunction::Fraction(fraction)
            }
        }
    }
}

impl GridTrack {
    fn into_taffy_track(self, scale_factor: f64) -> taffy::style::NonRepeatedTrackSizingFunction {
        let min = self.min_sizing_function.into_taffy(scale_factor);
        let max = self.max_sizing_function.into_taffy(scale_factor);
        taffy::style_helpers::minmax(min, max)
    }
}

impl RepeatedGridTrack {
    fn into_repeated_taffy_track(self, scale_factor: f64) -> taffy::style::TrackSizingFunction {
        let min = self.min_sizing_function.into_taffy(scale_factor);
        let max = self.max_sizing_function.into_taffy(scale_factor);
        let taffy_track: taffy::style::NonRepeatedTrackSizingFunction =
            taffy::style_helpers::minmax(min, max);
        match self.repetition {
            GridTrackRepetition::Count(count) => {
                if count == 1 {
                    taffy::style::TrackSizingFunction::Single(taffy_track)
                } else {
                    taffy::style_helpers::repeat(count, vec![taffy_track])
                }
            }
            GridTrackRepetition::AutoFit => taffy::style_helpers::repeat(
                taffy::style::GridTrackRepetition::AutoFit,
                vec![taffy_track],
            ),
            GridTrackRepetition::AutoFill => taffy::style_helpers::repeat(
                taffy::style::GridTrackRepetition::AutoFill,
                vec![taffy_track],
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_from() {
        use taffy::style_helpers as sh;

        let bevy_style = crate::Style {
            display: Display::Flex,
            position_type: PositionType::Absolute,
            left: Val::Px(0.),
            right: Val::Percent(0.),
            top: Val::Auto,
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
                left: Val::Percent(0.),
                right: Val::Px(0.),
                top: Val::Auto,
                bottom: Val::Auto,
            },
            padding: UiRect {
                left: Val::Percent(0.),
                right: Val::Px(0.),
                top: Val::Percent(0.),
                bottom: Val::Percent(0.),
            },
            border: UiRect {
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Auto,
                bottom: Val::Px(0.),
            },
            flex_grow: 1.,
            flex_shrink: 0.,
            flex_basis: Val::Px(0.),
            size: Size {
                width: Val::Px(0.),
                height: Val::Auto,
            },
            min_size: Size {
                width: Val::Px(0.),
                height: Val::Percent(0.),
            },
            max_size: Size {
                width: Val::Auto,
                height: Val::Px(0.),
            },
            aspect_ratio: None,
            overflow: crate::Overflow::Hidden,
            gap: Size {
                width: Val::Px(0.),
                height: Val::Percent(0.),
            },
            grid_auto_flow: GridAutoFlow::ColumnDense,
            grid_template_rows: vec![
                GridTrack::px(10.0),
                GridTrack::percent(50.0),
                GridTrack::fr(1.0),
            ],
            grid_template_columns: vec![GridTrack::px::<GridTrack>(10.0).repeat(5)],
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
        let taffy_style = from_style(1.0, &bevy_style);
        assert_eq!(taffy_style.display, taffy::style::Display::Flex);
        assert_eq!(taffy_style.position, taffy::style::Position::Absolute);
        assert!(matches!(
            taffy_style.inset.left,
            taffy::style::LengthPercentageAuto::Points(_)
        ));
        assert!(matches!(
            taffy_style.inset.right,
            taffy::style::LengthPercentageAuto::Percent(_)
        ));
        assert!(matches!(
            taffy_style.inset.top,
            taffy::style::LengthPercentageAuto::Auto
        ));
        assert!(matches!(
            taffy_style.inset.bottom,
            taffy::style::LengthPercentageAuto::Auto
        ));
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
        assert!(matches!(
            taffy_style.margin.left,
            taffy::style::LengthPercentageAuto::Percent(_)
        ));
        assert!(matches!(
            taffy_style.margin.right,
            taffy::style::LengthPercentageAuto::Points(_)
        ));
        assert!(matches!(
            taffy_style.margin.top,
            taffy::style::LengthPercentageAuto::Auto
        ));
        assert!(matches!(
            taffy_style.margin.bottom,
            taffy::style::LengthPercentageAuto::Auto
        ));
        assert!(matches!(
            taffy_style.padding.left,
            taffy::style::LengthPercentage::Percent(_)
        ));
        assert!(matches!(
            taffy_style.padding.right,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.padding.top,
            taffy::style::LengthPercentage::Percent(_)
        ));
        assert!(matches!(
            taffy_style.padding.bottom,
            taffy::style::LengthPercentage::Percent(_)
        ));
        assert!(matches!(
            taffy_style.border.left,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.border.right,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.border.top,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert!(matches!(
            taffy_style.border.bottom,
            taffy::style::LengthPercentage::Points(_)
        ));
        assert_eq!(taffy_style.flex_grow, 1.);
        assert_eq!(taffy_style.flex_shrink, 0.);
        assert!(matches!(
            taffy_style.flex_basis,
            taffy::style::Dimension::Points(_)
        ));
        assert!(matches!(
            taffy_style.size.width,
            taffy::style::Dimension::Points(_)
        ));
        assert!(matches!(
            taffy_style.size.height,
            taffy::style::Dimension::Auto
        ));
        assert!(matches!(
            taffy_style.min_size.width,
            taffy::style::Dimension::Points(_)
        ));
        assert!(matches!(
            taffy_style.min_size.height,
            taffy::style::Dimension::Percent(_)
        ));
        assert!(matches!(
            taffy_style.max_size.width,
            taffy::style::Dimension::Auto
        ));
        assert!(matches!(
            taffy_style.max_size.height,
            taffy::style::Dimension::Points(_)
        ));
        assert_eq!(taffy_style.aspect_ratio, None);
        assert_eq!(
            taffy_style.gap.width,
            taffy::style::LengthPercentage::Points(0.)
        );
        assert_eq!(
            taffy_style.gap.height,
            taffy::style::LengthPercentage::Percent(0.)
        );
        assert_eq!(
            taffy_style.grid_auto_flow,
            taffy::style::GridAutoFlow::ColumnDense
        );
        assert_eq!(
            taffy_style.grid_template_rows,
            vec![sh::points(10.0), sh::percent(0.5), sh::fr(1.0)]
        );
        assert_eq!(
            taffy_style.grid_template_columns,
            vec![sh::repeat(5, vec![sh::points(10.0)])]
        );
        assert_eq!(
            taffy_style.grid_auto_rows,
            vec![
                sh::fit_content(taffy::style::LengthPercentage::Points(10.0)),
                sh::fit_content(taffy::style::LengthPercentage::Percent(0.25)),
                sh::minmax(sh::points(0.0), sh::fr(2.0)),
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
        assert_eq!(
            taffy_style.grid_row,
            taffy::geometry::Line {
                start: sh::auto(),
                end: sh::span(3)
            }
        );
    }
}
