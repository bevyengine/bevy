use std::ops::{Deref, DerefMut};

use crate::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Display, FlexDirection, FlexWrap, GridAutoFlow,
    GridPlacement, GridTrack, JustifyContent, JustifyItems, JustifySelf, Node, Overflow,
    OverflowClipMargin, PositionType, RepeatedGridTrack, UiRect, Val,
};

/// A shared [`Node`] builder API that can be auto-implemented for types that can deref to `Node`
pub trait WidgetNodeBuilder {
    fn display(self, input: Display) -> Self;
    fn box_sizing(self, input: BoxSizing) -> Self;
    fn position_type(self, input: PositionType) -> Self;
    fn left(self, input: Val) -> Self;
    fn right(self, input: Val) -> Self;
    fn top(self, input: Val) -> Self;
    fn bottom(self, input: Val) -> Self;
    fn flex_direction(self, input: FlexDirection) -> Self;
    fn flex_wrap(self, input: FlexWrap) -> Self;
    fn align_items(self, input: AlignItems) -> Self;
    fn justify_items(self, input: JustifyItems) -> Self;
    fn align_self(self, input: AlignSelf) -> Self;
    fn justify_self(self, input: JustifySelf) -> Self;
    fn align_content(self, input: AlignContent) -> Self;
    fn justify_content(self, input: JustifyContent) -> Self;
    fn margin(self, input: UiRect) -> Self;
    fn padding(self, input: UiRect) -> Self;
    fn border(self, input: UiRect) -> Self;
    fn flex_grow(self, input: f32) -> Self;
    fn flex_shrink(self, input: f32) -> Self;
    fn flex_basis(self, input: Val) -> Self;
    fn width(self, input: Val) -> Self;
    fn height(self, input: Val) -> Self;
    fn min_width(self, input: Val) -> Self;
    fn min_height(self, input: Val) -> Self;
    fn max_width(self, input: Val) -> Self;
    fn max_height(self, input: Val) -> Self;
    fn aspect_ratio(self, input: Option<f32>) -> Self;
    fn overflow(self, input: Overflow) -> Self;
    fn overflow_clip_margin(self, input: OverflowClipMargin) -> Self;
    fn row_gap(self, input: Val) -> Self;
    fn column_gap(self, input: Val) -> Self;
    fn grid_auto_flow(self, input: GridAutoFlow) -> Self;
    fn grid_template_rows(self, input: Vec<RepeatedGridTrack>) -> Self;
    fn grid_template_columns(self, input: Vec<RepeatedGridTrack>) -> Self;
    fn grid_auto_rows(self, input: Vec<GridTrack>) -> Self;
    fn grid_auto_columns(self, input: Vec<GridTrack>) -> Self;
    fn grid_column(self, input: GridPlacement) -> Self;
    fn grid_row(self, input: GridPlacement) -> Self;
}

impl<T: Deref<Target = Node> + DerefMut> WidgetNodeBuilder for T {
    fn display(mut self, input: Display) -> Self {
        self.deref_mut().display = input;
        self
    }

    fn box_sizing(mut self, input: BoxSizing) -> Self {
        self.deref_mut().box_sizing = input;
        self
    }

    fn position_type(mut self, input: PositionType) -> Self {
        self.deref_mut().position_type = input;
        self
    }

    fn left(mut self, input: Val) -> Self {
        self.deref_mut().left = input;
        self
    }

    fn right(mut self, input: Val) -> Self {
        self.deref_mut().right = input;
        self
    }

    fn top(mut self, input: Val) -> Self {
        self.deref_mut().top = input;
        self
    }

    fn bottom(mut self, input: Val) -> Self {
        self.deref_mut().bottom = input;
        self
    }

    fn flex_direction(mut self, input: FlexDirection) -> Self {
        self.deref_mut().flex_direction = input;
        self
    }

    fn flex_wrap(mut self, input: FlexWrap) -> Self {
        self.deref_mut().flex_wrap = input;
        self
    }

    fn align_items(mut self, input: AlignItems) -> Self {
        self.deref_mut().align_items = input;
        self
    }

    fn justify_items(mut self, input: JustifyItems) -> Self {
        self.deref_mut().justify_items = input;
        self
    }

    fn align_self(mut self, input: AlignSelf) -> Self {
        self.deref_mut().align_self = input;
        self
    }

    fn justify_self(mut self, input: JustifySelf) -> Self {
        self.deref_mut().justify_self = input;
        self
    }

    fn align_content(mut self, input: AlignContent) -> Self {
        self.deref_mut().align_content = input;
        self
    }

    fn justify_content(mut self, input: JustifyContent) -> Self {
        self.deref_mut().justify_content = input;
        self
    }

    fn margin(mut self, input: UiRect) -> Self {
        self.deref_mut().margin = input;
        self
    }

    fn padding(mut self, input: UiRect) -> Self {
        self.deref_mut().padding = input;
        self
    }

    fn border(mut self, input: UiRect) -> Self {
        self.deref_mut().border = input;
        self
    }

    fn flex_grow(mut self, input: f32) -> Self {
        self.deref_mut().flex_grow = input;
        self
    }

    fn flex_shrink(mut self, input: f32) -> Self {
        self.deref_mut().flex_shrink = input;
        self
    }

    fn flex_basis(mut self, input: Val) -> Self {
        self.deref_mut().flex_basis = input;
        self
    }

    fn width(mut self, input: Val) -> Self {
        self.deref_mut().width = input;
        self
    }

    fn height(mut self, input: Val) -> Self {
        self.deref_mut().height = input;
        self
    }

    fn min_width(mut self, input: Val) -> Self {
        self.deref_mut().min_width = input;
        self
    }

    fn min_height(mut self, input: Val) -> Self {
        self.deref_mut().min_height = input;
        self
    }

    fn max_width(mut self, input: Val) -> Self {
        self.deref_mut().max_width = input;
        self
    }

    fn max_height(mut self, input: Val) -> Self {
        self.deref_mut().max_height = input;
        self
    }

    fn aspect_ratio(mut self, input: Option<f32>) -> Self {
        self.deref_mut().aspect_ratio = input;
        self
    }

    fn overflow(mut self, input: Overflow) -> Self {
        self.deref_mut().overflow = input;
        self
    }

    fn overflow_clip_margin(mut self, input: OverflowClipMargin) -> Self {
        self.deref_mut().overflow_clip_margin = input;
        self
    }

    fn row_gap(mut self, input: Val) -> Self {
        self.deref_mut().row_gap = input;
        self
    }

    fn column_gap(mut self, input: Val) -> Self {
        self.deref_mut().column_gap = input;
        self
    }

    fn grid_auto_flow(mut self, input: GridAutoFlow) -> Self {
        self.deref_mut().grid_auto_flow = input;
        self
    }

    fn grid_template_rows(mut self, input: Vec<RepeatedGridTrack>) -> Self {
        self.deref_mut().grid_template_rows = input;
        self
    }

    fn grid_template_columns(mut self, input: Vec<RepeatedGridTrack>) -> Self {
        self.deref_mut().grid_template_columns = input;
        self
    }

    fn grid_auto_rows(mut self, input: Vec<GridTrack>) -> Self {
        self.deref_mut().grid_auto_rows = input;
        self
    }

    fn grid_auto_columns(mut self, input: Vec<GridTrack>) -> Self {
        self.deref_mut().grid_auto_columns = input;
        self
    }

    fn grid_column(mut self, input: GridPlacement) -> Self {
        self.deref_mut().grid_column = input;
        self
    }

    fn grid_row(mut self, input: GridPlacement) -> Self {
        self.deref_mut().grid_row = input;
        self
    }
}
