/// Helper struct used to process an iterator of `Result<Vec<T>, syn::Error>`,
/// combining errors into one along the way.
pub(crate) struct ResultSifter<T> {
    items: Vec<T>,
    errors: Option<syn::Error>,
}

impl<T> Default for ResultSifter<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            errors: None,
        }
    }
}

impl<T> ResultSifter<T> {
    /// Sift the given result, combining errors if necessary.
    pub fn sift(&mut self, result: Result<T, syn::Error>) {
        match result {
            Ok(data) => self.items.push(data),
            Err(err) => {
                if let Some(ref mut errors) = self.errors {
                    errors.combine(err);
                } else {
                    self.errors = Some(err);
                }
            }
        }
    }

    /// Associated method that provides a convenient implementation for [`Iterator::fold`].
    pub fn fold(mut sifter: Self, result: Result<T, syn::Error>) -> Self {
        sifter.sift(result);
        sifter
    }

    /// Complete the sifting process and return the final result.
    pub fn finish(self) -> Result<Vec<T>, syn::Error> {
        if let Some(errors) = self.errors {
            Err(errors)
        } else {
            Ok(self.items)
        }
    }
}
