use argh::FromArgs;
use bevy::prelude::Resource;

#[derive(FromArgs, Resource)]
/// `many_buttons` general UI benchmark that stress tests layouting, text, interaction and rendering
pub(crate) struct Args {
    /// whether to add text to each button
    #[argh(switch)]
    pub(crate) no_text: bool,

    /// whether to add borders to each button
    #[argh(switch)]
    pub(crate) no_borders: bool,

    /// whether to perform a full relayout each frame
    #[argh(switch)]
    pub(crate) relayout: bool,

    /// whether to recompute all text each frame
    #[argh(switch)]
    pub(crate) recompute_text: bool,

    /// how many buttons per row and column of the grid.
    #[argh(option, default = "110")]
    pub(crate) buttons: usize,

    /// give every nth button an image
    #[argh(option, default = "4")]
    pub(crate) image_freq: usize,

    /// use the grid layout model
    #[argh(switch)]
    pub(crate) grid: bool,
}
