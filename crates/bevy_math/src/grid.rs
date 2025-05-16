use core::ops::{Index, IndexMut};
use glam::UVec2;

#[cfg(feature = "alloc")]
use {alloc::vec, alloc::vec::Vec};

use crate::URect;

pub struct Grid<T> {
    rows: usize,
    columns: usize,
    data: Vec<T>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GridError {
    /// The given indices were out of bounds.
    IndicesOutOfBounds(usize, usize),
    /// The given index in row or column major order was out of bounds.
    IndexOutOfBounds(usize),
}

impl<T> Grid<T> {
    pub fn new(rows: usize, columns: usize) -> Self
    where
        T: Clone + Default,
    {
        Self::filled_with(T::default(), rows, columns)
    }

    pub fn filled_with(element: T, rows: usize, columns: usize) -> Self
    where
        T: Clone,
    {
        Self {
            rows,
            columns,
            data: vec![element; rows * columns],
        }
    }

    pub fn filled_by<F>(mut f: F, rows: usize, columns: usize) -> Self
    where
        F: FnMut(usize, usize) -> T,
    {
        let data = (0..rows * columns)
            .map(|i| f(i % columns, i / rows))
            .collect();
        Self {
            rows,
            columns,
            data,
        }
    }

    pub fn get(&self, column: usize, row: usize) -> Option<&T> {
        self.index(column, row).map(|i| &self.data[i])
    }

    pub fn get_mut(&mut self, column: usize, row: usize) -> Option<&mut T> {
        self.index(column, row).map(|i| &mut self.data[i])
    }

    pub fn set(&mut self, column: usize, row: usize, value: T) -> Result<(), GridError> {
        self.get_mut(column, row)
            .map(|location| {
                *location = value;
            })
            .ok_or(GridError::IndicesOutOfBounds(column, row))
    }

    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        self.data.fill(value);
    }

    pub fn fill_with<F>(&mut self, f: F)
    where
        F: FnMut() -> T,
    {
        self.data.fill_with(f);
    }

    pub fn sub_rect<'a>(
        &'a self,
        rect: impl Into<URect>,
    ) -> Result<impl Iterator<Item = &'a T>, GridError> {
        let rect: URect = rect.into();
        let width = rect.size().x as usize;
        if rect.max.x >= self.columns as u32 || rect.max.y >= self.rows as u32 {
            return Err(GridError::IndicesOutOfBounds(
                rect.min.x as usize,
                rect.min.y as usize,
            ));
        }
        Ok((rect.min.y..rect.max.y).flat_map(move |row| {
            let start = row as usize * self.columns + rect.min.x as usize;
            let end = start + width;
            self.data[start..end].iter()
        }))
    }

    pub fn size(&self) -> (usize, usize) {
        (self.rows, self.columns)
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    fn index(&self, column: usize, row: usize) -> Option<usize> {
        if column >= self.columns || row >= self.rows {
            None
        } else {
            Some(row * self.columns + column)
        }
    }
}

impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        self.get(x, y)
            .unwrap_or_else(|| panic!("Index out of bounds: {:?}", (x, y)))
    }
}

impl<T> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        self.get_mut(x, y)
            .unwrap_or_else(|| panic!("Index out of bounds: {:?}", (x, y)))
    }
}

impl<T> Index<UVec2> for Grid<T> {
    type Output = T;

    fn index(&self, pos: UVec2) -> &Self::Output {
        self.get(pos.x as usize, pos.y as usize)
            .unwrap_or_else(|| panic!("Index out of bounds: {:?}", pos))
    }
}

impl<T> IndexMut<UVec2> for Grid<T> {
    fn index_mut(&mut self, pos: UVec2) -> &mut Self::Output {
        self.get_mut(pos.x as usize, pos.y as usize)
            .unwrap_or_else(|| panic!("Index out of bounds: {:?}", pos))
    }
}
