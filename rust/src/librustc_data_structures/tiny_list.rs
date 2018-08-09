// Copyright 2018 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


//! A singly-linked list.
//!
//! Using this data structure only makes sense under very specific
//! circumstances:
//!
//! - If you have a list that rarely stores more than one element, then this
//!   data-structure can store the element without allocating and only uses as
//!   much space as a `Option<(T, usize)>`. If T can double as the `Option`
//!   discriminant, it will even only be as large as `T, usize`.
//!
//! If you expect to store more than 1 element in the common case, steer clear
//! and use a `Vec<T>`, `Box<[T]>`, or a `SmallVec<T>`.

use std::mem;

#[derive(Clone, Hash, Debug, PartialEq)]
pub struct TinyList<T: PartialEq> {
    head: Option<Element<T>>
}

impl<T: PartialEq> TinyList<T> {

    #[inline]
    pub fn new() -> TinyList<T> {
        TinyList {
            head: None
        }
    }

    #[inline]
    pub fn new_single(data: T) -> TinyList<T> {
        TinyList {
            head: Some(Element {
                data,
                next: None,
            })
        }
    }

    #[inline]
    pub fn insert(&mut self, data: T) {
        let current_head = mem::replace(&mut self.head, None);

        if let Some(current_head) = current_head {
            let current_head = Box::new(current_head);
            self.head = Some(Element {
                data,
                next: Some(current_head)
            });
        } else {
            self.head = Some(Element {
                data,
                next: None,
            })
        }
    }

    #[inline]
    pub fn remove(&mut self, data: &T) -> bool {
        let remove_head = if let Some(ref mut head) = self.head {
            if head.data == *data {
                Some(mem::replace(&mut head.next, None))
            } else {
                None
            }
        } else {
            return false
        };

        if let Some(remove_head) = remove_head {
            if let Some(next) = remove_head {
                self.head = Some(*next);
            } else {
                self.head = None;
            }
            return true
        }

        self.head.as_mut().unwrap().remove_next(data)
    }

    #[inline]
    pub fn contains(&self, data: &T) -> bool {
        if let Some(ref head) = self.head {
            head.contains(data)
        } else {
            false
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        if let Some(ref head) = self.head {
            head.len()
        } else {
            0
        }
    }
}

#[derive(Clone, Hash, Debug, PartialEq)]
struct Element<T: PartialEq> {
    data: T,
    next: Option<Box<Element<T>>>,
}

impl<T: PartialEq> Element<T> {

    fn remove_next(&mut self, data: &T) -> bool {
        let new_next = if let Some(ref mut next) = self.next {
            if next.data != *data {
                return next.remove_next(data)
            } else {
                mem::replace(&mut next.next, None)
            }
        } else {
            return false
        };

        self.next = new_next;
        return true
    }

    fn len(&self) -> usize {
        if let Some(ref next) = self.next {
            1 + next.len()
        } else {
            1
        }
    }

    fn contains(&self, data: &T) -> bool {
        if self.data == *data {
            return true
        }

        if let Some(ref next) = self.next {
            next.contains(data)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_contains_and_insert() {
        fn do_insert(i : u32) -> bool {
            i % 2 == 0
        }

        let mut list = TinyList::new();

        for i in 0 .. 10 {
            for j in 0 .. i {
                if do_insert(j) {
                    assert!(list.contains(&j));
                } else {
                    assert!(!list.contains(&j));
                }
            }

            assert!(!list.contains(&i));

            if do_insert(i) {
                list.insert(i);
                assert!(list.contains(&i));
            }
        }
    }

    #[test]
    fn test_remove_first() {
        let mut list = TinyList::new();
        list.insert(1);
        list.insert(2);
        list.insert(3);
        list.insert(4);
        assert_eq!(list.len(), 4);

        assert!(list.remove(&4));
        assert!(!list.contains(&4));

        assert_eq!(list.len(), 3);
        assert!(list.contains(&1));
        assert!(list.contains(&2));
        assert!(list.contains(&3));
    }

    #[test]
    fn test_remove_last() {
        let mut list = TinyList::new();
        list.insert(1);
        list.insert(2);
        list.insert(3);
        list.insert(4);
        assert_eq!(list.len(), 4);

        assert!(list.remove(&1));
        assert!(!list.contains(&1));

        assert_eq!(list.len(), 3);
        assert!(list.contains(&2));
        assert!(list.contains(&3));
        assert!(list.contains(&4));
    }

    #[test]
    fn test_remove_middle() {
        let mut list = TinyList::new();
        list.insert(1);
        list.insert(2);
        list.insert(3);
        list.insert(4);
        assert_eq!(list.len(), 4);

        assert!(list.remove(&2));
        assert!(!list.contains(&2));

        assert_eq!(list.len(), 3);
        assert!(list.contains(&1));
        assert!(list.contains(&3));
        assert!(list.contains(&4));
    }

    #[test]
    fn test_remove_single() {
        let mut list = TinyList::new();
        list.insert(1);
        assert_eq!(list.len(), 1);

        assert!(list.remove(&1));
        assert!(!list.contains(&1));

        assert_eq!(list.len(), 0);
    }
}
