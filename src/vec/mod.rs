pub struct Vec<T> {
    items: Box<[T]>,
    len: usize,
    cap: usize,
}

impl<T> Vec<T> {
    pub fn new() -> Vec<T> {
        Vec {
            items: Box::new([]),
            len: 0,
            cap: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn index(&self, i: usize) -> Option<&T> {
        if self.len == 0 || i > self.len - 1 {
            return None;
        }
        Some(&(*self.items)[i])
    }

    pub fn push(&mut self, item: T) {
        if self.len + 1 > self.cap {
            // realloc (aka grow)
        }
        (*self.items)[self.len] = item;
        self.len = self.len + 1;
    }
}

#[cfg(test)]
mod test;
