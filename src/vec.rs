pub struct Vec<T> {
    items: Box<[T]>,
    len: usize,
    cap: usize,
}

struct Nothing;

pub fn new<T>() -> Vec<T> {
    return Vec {
        items: Box::new([]),
        len: 0,
        cap: 0,
    };
}

impl<T> Vec<T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn index(&self, i: usize) -> Option<&T> {
        if self.len == 0 || i > self.len - 1 {
            return None;
        }
        Some(&(*self.items)[i])
    }
}

#[cfg(test)]
mod test;
