use bevy_ecs::resource::Resource;
use std::borrow::Cow;

/// Resource providing access to the clipboard
#[derive(Resource, Default)]
pub struct Clipboard;

impl Clipboard {
    fn new() -> Result<arboard::Clipboard, arboard::Error> {
        Err(arboard::Error::ClipboardNotSupported)
    }

    /// Fetches UTF-8 text from the clipboard and returns it.
    ///
    /// # Errors
    ///
    /// Returns error if clipboard is empty or contents are not UTF-8 text.
    pub fn get_text(&mut self) -> Result<String, arboard::Error> {
        Err(arboard::Error::ClipboardNotSupported)
    }

    /// Places the text onto the clipboard. Any valid UTF-8 string is accepted.
    ///
    /// # Errors
    ///
    /// Returns error if `text` failed to be stored on the clipboard.
    pub fn set_text<'a, T: Into<Cow<'a, str>>>(&mut self, text: T) -> Result<(), arboard::Error> {
        Err(arboard::Error::ClipboardNotSupported)
    }
}
