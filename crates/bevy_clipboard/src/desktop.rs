extern crate alloc;

use bevy_ecs::resource::Resource;

/// Resource providing access to the clipboard
#[derive(Resource, Default)]
pub struct Clipboard;

impl Clipboard {
    fn new() -> Result<arboard::Clipboard, arboard::Error> {
        arboard::Clipboard::new()
    }

    /// Fetches UTF-8 text from the clipboard and returns it.
    ///
    /// # Errors
    ///
    /// Returns error if clipboard is empty or contents are not UTF-8 text.
    pub fn get_text(&mut self) -> Result<String, arboard::Error> {
        Self::new().and_then(|mut clipboard| clipboard.get_text())
    }

    /// Places the text onto the clipboard. Any valid UTF-8 string is accepted.
    ///
    /// # Errors
    ///
    /// Returns error if `text` failed to be stored on the clipboard.
    pub fn set_text<'a, T: Into<alloc::borrow::Cow<'a, str>>>(
        &mut self,
        text: T,
    ) -> Result<(), arboard::Error> {
        Self::new().and_then(|mut clipboard| clipboard.set_text(text))
    }
}
