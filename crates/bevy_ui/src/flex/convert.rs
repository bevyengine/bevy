use taffy::style::LengthPercentageAuto;
use taffy::style_helpers;

use crate::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, GridAutoFlow,
    GridPlacement, GridTrack, JustifyContent, JustifyItems, JustifySelf, MaxTrackSizingFunction,
    MinTrackSizingFunction, PositionType, Size, Style, UiRect, Val,
};

impl Val {
    fn scaled(self, scale_factor: f64) -> Self {
        match self {
            Val::Auto => Val::Auto,
            Val::Percent(value) => Val::Percent(value),
            Val::Px(value) => Val::Px((scale_factor * value as f64) as f32),
            Val::Undefined => Val::Undefined,
        }
    }

    fn to_inset(self) -> LengthPercentageAuto {
        match self {
            Val::Auto | Val::Undefined => taffy::style::LengthPercentageAuto::Auto,
            Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentageAuto::Points(value),
        }
    }
}

impl UiRect {
    fn scaled(self, scale_factor: f64) -> Self {
        Self {
            left: self.left.scaled(scale_factor),
            right: self.right.scaled(scale_factor),
            top: self.top.scaled(scale_factor),
            bottom: self.bottom.scaled(scale_factor),
        }
    }
}

impl Size {
    fn scaled(self, scale_factor: f64) -> Self {
        Self {
            width: self.width.scaled(scale_factor),
            height: self.height.scaled(scale_factor),
        }
    }
}

impl<T: From<Val>> From<UiRect> for taffy::prelude::Rect<T> {
    fn from(value: UiRect) -> Self {
        Self {
            left: value.left.into(),
            right: value.right.into(),
            top: value.top.into(),
            bottom: value.bottom.into(),
        }
    }
}

impl<T: From<Val>> From<Size> for taffy::prelude::Size<T> {
    fn from(value: Size) -> Self {
        Self {
            width: value.width.into(),
            height: value.height.into(),
        }
    }
}

impl From<Val> for taffy::style::Dimension {
    fn from(value: Val) -> Self {
        match value {
            Val::Auto | Val::Undefined => taffy::style::Dimension::Auto,
            Val::Percent(value) => taffy::style::Dimension::Percent(value / 100.0),
            Val::Px(value) => taffy::style::Dimension::Points(value),
        }
    }
}

impl From<Val> for taffy::style::LengthPercentage {
    fn from(value: Val) -> Self {
        match value {
            Val::Auto | Val::Undefined => taffy::style::LengthPercentage::Points(0.0),
            Val::Percent(value) => taffy::style::LengthPercentage::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentage::Points(value),
        }
    }
}

impl From<Val> for taffy::style::LengthPercentageAuto {
    fn from(value: Val) -> Self {
        match value {
            Val::Auto => taffy::style::LengthPercentageAuto::Auto,
            Val::Percent(value) => taffy::style::LengthPercentageAuto::Percent(value / 100.0),
            Val::Px(value) => taffy::style::LengthPercentageAuto::Points(value),
            Val::Undefined => taffy::style::LengthPercentageAuto::Points(0.),
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
            left: style.position.left.scaled(scale_factor).to_inset(),
            right: style.position.right.scaled(scale_factor).to_inset(),
            top: style.position.top.scaled(scale_factor).to_inset(),
            bottom: style.position.bottom.scaled(scale_factor).to_inset(),
        },
        margin: style.margin.scaled(scale_factor).into(),
        padding: style.padding.scaled(scale_factor).into(),
        border: style.border.scaled(scale_factor).into(),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis.scaled(scale_factor).into(),
        size: style.size.scaled(scale_factor).into(),
        min_size: style.min_size.scaled(scale_factor).into(),
        max_size: style.max_size.scaled(scale_factor).into(),
        aspect_ratio: style.aspect_ratio,
        gap: style.gap.scaled(scale_factor).into(),
        grid_auto_flow: style.grid_auto_flow.into(),
        grid_template_rows: style.grid_template_rows.into(),
        grid_template_columns: style.grid_template_columns.into(),
        grid_auto_rows: style.grid_auto_rows.into(),
        grid_auto_columns: style.grid_auto_columns.into(),
        grid_row: style.grid_row.into(),
        grid_column: style.grid_column.into(),
    }
}

impl From<AlignItems> for Option<taffy::style::AlignItems> {
    fn from(value: AlignItems) -> Self {
        match value {
            AlignItems::Normal => None,
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
            JustifyItems::Normal => None,
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
            AlignContent::Normal => None,
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
            JustifyContent::Normal => None,
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

impl From<MinTrackSizingFunction> for taffy::style::MinTrackSizingFunction {
    fn from(value: MinTrackSizingFunction) -> Self {
        match value {
            MinTrackSizingFunction::Fixed(val) => {
                taffy::style::MinTrackSizingFunction::Fixed(val.into())
            }
            MinTrackSizingFunction::Auto => taffy::style::MinTrackSizingFunction::Auto,
            MinTrackSizingFunction::MinContent => taffy::style::MinTrackSizingFunction::MinContent,
            MinTrackSizingFunction::MaxContent => taffy::style::MinTrackSizingFunction::MaxContent,
        }
    }
}

impl From<MaxTrackSizingFunction> for taffy::style::MaxTrackSizingFunction {
    fn from(value: MaxTrackSizingFunction) -> Self {
        match value {
            MaxTrackSizingFunction::Fixed(val) => {
                taffy::style::MaxTrackSizingFunction::Fixed(val.into())
            }
            MaxTrackSizingFunction::Auto => taffy::style::MaxTrackSizingFunction::Auto,
            MaxTrackSizingFunction::MinContent => taffy::style::MaxTrackSizingFunction::MinContent,
            MaxTrackSizingFunction::MaxContent => taffy::style::MaxTrackSizingFunction::MaxContent,
            MaxTrackSizingFunction::FitContent(val) => {
                taffy::style::MaxTrackSizingFunction::FitContent(val.into())
            }
            MaxTrackSizingFunction::Fraction(fraction) => {
                taffy::style::MaxTrackSizingFunction::Fraction(fraction)
            }
        }
    }
}

impl From<GridTrack> for taffy::style::NonRepeatedTrackSizingFunction {
    fn from(value: GridTrack) -> Self {
        let min = value.min_sizing_function.into();
        let max = value.max_sizing_function.into();
        taffy::style_helpers::minmax(min, max)
    }
}

impl From<GridTrack> for taffy::style::TrackSizingFunction {
    fn from(value: GridTrack) -> Self {
        let min = value.min_sizing_function.into();
        let max = value.max_sizing_function.into();
        taffy::style_helpers::minmax(min, max)
    }
}
