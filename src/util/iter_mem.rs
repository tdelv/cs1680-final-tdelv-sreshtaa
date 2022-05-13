use std::{cell::RefCell, rc::Rc};

struct IterRoot<I>
where
    I: Iterator
{
    iter: I,
    mem: Vec<Rc<I::Item>>,
    end: bool
}

impl<I> IterRoot<I>
where
    I: Iterator
{
    fn new(iter: I) -> Self {
        Self {
            iter,
            mem: Vec::new(),
            end: false
        }
    }

    fn ensure(&mut self, n: usize) {
        if self.end {
            return;
        }

        while self.mem.len() <= n {
            if let Some(next) = self.iter.next() {
                self.mem.push(Rc::new(next))
            } else {
                self.end = true;
                return;
            }
        }
    }

    fn get(&mut self, n: usize) -> Option<Rc<I::Item>> {
        self.ensure(n);
        self.mem.get(n).map(Rc::clone)
    }

    fn get_range(&mut self, start: usize, end: usize) -> Vec<Rc<I::Item>> {
        (start..end).map_while(|i| self.get(i)).collect()
    }
}

pub struct IterMem<I>
where
    I: Iterator
{
    root: Rc<RefCell<IterRoot<I>>>,
    index: usize
}

impl<I> Clone for IterMem<I>
where
    I: Iterator
{
    fn clone(&self) -> Self {
        Self { 
            root: Rc::clone(&self.root), 
            index: self.index 
        }
    }
}

impl<I> IterMem<I>
where
    I: Iterator
{
    pub fn new(iter: I) -> Self {
        Self {
            root: Rc::new(RefCell::new(IterRoot::new(iter))),
            index: 0
        }
    }

    pub fn peek(&self, n: usize) -> Vec<Rc<I::Item>> {
        self.root.borrow_mut().get_range(self.index, self.index + n)
    }
}

impl<I> Iterator for IterMem<I>
where
    I: Iterator
{
    type Item = Rc<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.root.borrow_mut().get(self.index);
        self.index += 1;
        ret
    }
}
