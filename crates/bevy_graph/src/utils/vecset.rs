/// Set-like methods for `Vec<T>`
pub trait VecSet<T: PartialEq> {
    fn index_by_value(&self, value: &T) -> Option<usize>;
    fn remove_by_value(&mut self, value: &T) -> Option<T>;
}

impl<T: PartialEq> VecSet<T> for Vec<T> {
    fn index_by_value(&self, value: &T) -> Option<usize> {
        self.iter().position(|l| l == value)
    }

    fn remove_by_value(&mut self, value: &T) -> Option<T> {
        if let Some(index) = self.index_by_value(value) {
            Some(self.remove(index))
        } else {
            None
        }
    }
}
