use super::{BatchKey, Key};

#[derive(Debug, Eq, PartialEq)]
pub struct Batch<TKey, TValue, TData>
where
    TKey: Key,
{
    pub batch_key: BatchKey<TKey>,
    pub values: Vec<TValue>,
    pub data: TData,
}

impl<TKey, TValue, TData> Batch<TKey, TValue, TData>
where
    TKey: Key,
{
    pub fn new(batch_key: BatchKey<TKey>, data: TData) -> Self {
        Batch {
            data,
            values: Vec::new(),
            batch_key,
        }
    }

    pub fn add(&mut self, value: TValue) {
        self.values.push(value);
    }

    pub fn iter(&self) -> impl Iterator<Item = &TValue> {
        self.values.iter()
    }

    pub fn get_key(&self, index: usize) -> Option<&TKey> {
        self.batch_key.0.get(index)
    }
}
