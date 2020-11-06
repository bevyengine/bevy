// This is a really basic newtype wrapper since there _may_ be more that needs to be added eventually.
// Also, having an event that's just of `char` is not exactly the best idea...

// As well, unlike the other inputs, there's no system for syncing the events to the Input<T> type.
// This is because the "press-able" style of Input<T> doesn't map well to character/text inputs.

/// A character input event as sent by the OS or otherwise underlying system.
#[derive(Debug, Clone)]
pub struct CharInput(pub char);
