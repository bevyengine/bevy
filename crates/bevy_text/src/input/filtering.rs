use bevy_ecs::component::Component;

/// If a text input entity has a `TextInputFilter` component, after each [`TextEdit`] is applied, the [`TextInputBuffer`]’s text is checked
/// against the filter, and if it fails, the `TextEdit` is immediately rolled back, and a [`TextInputEvent::InvalidEdit`] event is emitted.
#[derive(Component)]
pub enum TextInputFilter {
    /// Positive integer input
    /// accepts only digits
    PositiveInteger,
    /// Integer input
    /// accepts only digits and a leading sign
    Integer,
    /// Decimal input
    /// accepts only digits, a decimal point and a leading sign
    Decimal,
    /// Hexadecimal input
    /// accepts only `0-9`, `a-f` and `A-F`
    Hex,
    /// Alphanumeric input
    /// accepts only `0-9`, `a-z` and `A-Z`
    Alphanumeric,
    /// Custom filter
    Custom(Box<dyn Fn(&str) -> bool + Send + Sync>),
}

impl TextInputFilter {
    /// Returns `true` if the given `text` passes the filter
    pub fn is_match(&self, text: &str) -> bool {
        // Always passes if the input is empty unless using a custom filter
        if text.is_empty() && !matches!(self, Self::Custom(_)) {
            return true;
        }

        match self {
            TextInputFilter::PositiveInteger => text.chars().all(|c| c.is_ascii_digit()),
            TextInputFilter::Integer => text
                .strip_prefix('-')
                .unwrap_or(text)
                .chars()
                .all(|c| c.is_ascii_digit()),
            TextInputFilter::Decimal => text
                .strip_prefix('-')
                .unwrap_or(text)
                .chars()
                .try_fold(true, |is_int, c| match c {
                    '.' if is_int => Ok(false),
                    c if c.is_ascii_digit() => Ok(is_int),
                    _ => Err(()),
                })
                .is_ok(),
            TextInputFilter::Hex => text.chars().all(|c| c.is_ascii_hexdigit()),
            TextInputFilter::Alphanumeric => text.chars().all(|c| c.is_ascii_alphanumeric()),
            TextInputFilter::Custom(is_match) => is_match(text),
        }
    }

    /// Create a custom filter
    pub fn custom(filter_fn: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        Self::Custom(Box::new(filter_fn))
    }
}
