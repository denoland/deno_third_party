// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use indexed_vec::{Idx, IndexVec};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::marker::PhantomData;

type Word = u128;
const WORD_BITS: usize = 128;

/// A very simple BitVector type.
#[derive(Clone, Debug, PartialEq)]
pub struct BitVector {
    data: Vec<Word>,
}

impl BitVector {
    #[inline]
    pub fn new(num_bits: usize) -> BitVector {
        let num_words = words(num_bits);
        BitVector {
            data: vec![0; num_words],
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        for p in &mut self.data {
            *p = 0;
        }
    }

    pub fn count(&self) -> usize {
        self.data.iter().map(|e| e.count_ones() as usize).sum()
    }

    #[inline]
    pub fn contains(&self, bit: usize) -> bool {
        let (word, mask) = word_mask(bit);
        (self.data[word] & mask) != 0
    }

    /// Returns true if the bit has changed.
    #[inline]
    pub fn insert(&mut self, bit: usize) -> bool {
        let (word, mask) = word_mask(bit);
        let data = &mut self.data[word];
        let value = *data;
        let new_value = value | mask;
        *data = new_value;
        new_value != value
    }

    /// Returns true if the bit has changed.
    #[inline]
    pub fn remove(&mut self, bit: usize) -> bool {
        let (word, mask) = word_mask(bit);
        let data = &mut self.data[word];
        let value = *data;
        let new_value = value & !mask;
        *data = new_value;
        new_value != value
    }

    #[inline]
    pub fn insert_all(&mut self, all: &BitVector) -> bool {
        assert!(self.data.len() == all.data.len());
        let mut changed = false;
        for (i, j) in self.data.iter_mut().zip(&all.data) {
            let value = *i;
            *i = value | *j;
            if value != *i {
                changed = true;
            }
        }
        changed
    }

    #[inline]
    pub fn grow(&mut self, num_bits: usize) {
        let num_words = words(num_bits);
        if self.data.len() < num_words {
            self.data.resize(num_words, 0)
        }
    }

    /// Iterates over indexes of set bits in a sorted order
    #[inline]
    pub fn iter<'a>(&'a self) -> BitVectorIter<'a> {
        BitVectorIter {
            iter: self.data.iter(),
            current: 0,
            idx: 0,
        }
    }
}

pub struct BitVectorIter<'a> {
    iter: ::std::slice::Iter<'a, Word>,
    current: Word,
    idx: usize,
}

impl<'a> Iterator for BitVectorIter<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        while self.current == 0 {
            self.current = if let Some(&i) = self.iter.next() {
                if i == 0 {
                    self.idx += WORD_BITS;
                    continue;
                } else {
                    self.idx = words(self.idx) * WORD_BITS;
                    i
                }
            } else {
                return None;
            }
        }
        let offset = self.current.trailing_zeros() as usize;
        self.current >>= offset;
        self.current >>= 1; // shift otherwise overflows for 0b1000_0000_…_0000
        self.idx += offset + 1;
        return Some(self.idx - 1);
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

impl FromIterator<bool> for BitVector {
    fn from_iter<I>(iter: I) -> BitVector
    where
        I: IntoIterator<Item = bool>,
    {
        let iter = iter.into_iter();
        let (len, _) = iter.size_hint();
        // Make the minimum length for the bitvector WORD_BITS bits since that's
        // the smallest non-zero size anyway.
        let len = if len < WORD_BITS { WORD_BITS } else { len };
        let mut bv = BitVector::new(len);
        for (idx, val) in iter.enumerate() {
            if idx > len {
                bv.grow(idx);
            }
            if val {
                bv.insert(idx);
            }
        }

        bv
    }
}

/// A "bit matrix" is basically a matrix of booleans represented as
/// one gigantic bitvector. In other words, it is as if you have
/// `rows` bitvectors, each of length `columns`.
#[derive(Clone, Debug)]
pub struct BitMatrix {
    columns: usize,
    vector: Vec<Word>,
}

impl BitMatrix {
    /// Create a new `rows x columns` matrix, initially empty.
    pub fn new(rows: usize, columns: usize) -> BitMatrix {
        // For every element, we need one bit for every other
        // element. Round up to an even number of words.
        let words_per_row = words(columns);
        BitMatrix {
            columns,
            vector: vec![0; rows * words_per_row],
        }
    }

    /// The range of bits for a given row.
    fn range(&self, row: usize) -> (usize, usize) {
        let words_per_row = words(self.columns);
        let start = row * words_per_row;
        (start, start + words_per_row)
    }

    /// Sets the cell at `(row, column)` to true. Put another way, add
    /// `column` to the bitset for `row`.
    ///
    /// Returns true if this changed the matrix, and false otherwise.
    pub fn add(&mut self, row: usize, column: usize) -> bool {
        let (start, _) = self.range(row);
        let (word, mask) = word_mask(column);
        let vector = &mut self.vector[..];
        let v1 = vector[start + word];
        let v2 = v1 | mask;
        vector[start + word] = v2;
        v1 != v2
    }

    /// Do the bits from `row` contain `column`? Put another way, is
    /// the matrix cell at `(row, column)` true?  Put yet another way,
    /// if the matrix represents (transitive) reachability, can
    /// `row` reach `column`?
    pub fn contains(&self, row: usize, column: usize) -> bool {
        let (start, _) = self.range(row);
        let (word, mask) = word_mask(column);
        (self.vector[start + word] & mask) != 0
    }

    /// Returns those indices that are true in rows `a` and `b`.  This
    /// is an O(n) operation where `n` is the number of elements
    /// (somewhat independent from the actual size of the
    /// intersection, in particular).
    pub fn intersection(&self, a: usize, b: usize) -> Vec<usize> {
        let (a_start, a_end) = self.range(a);
        let (b_start, b_end) = self.range(b);
        let mut result = Vec::with_capacity(self.columns);
        for (base, (i, j)) in (a_start..a_end).zip(b_start..b_end).enumerate() {
            let mut v = self.vector[i] & self.vector[j];
            for bit in 0..WORD_BITS {
                if v == 0 {
                    break;
                }
                if v & 0x1 != 0 {
                    result.push(base * WORD_BITS + bit);
                }
                v >>= 1;
            }
        }
        result
    }

    /// Add the bits from row `read` to the bits from row `write`,
    /// return true if anything changed.
    ///
    /// This is used when computing transitive reachability because if
    /// you have an edge `write -> read`, because in that case
    /// `write` can reach everything that `read` can (and
    /// potentially more).
    pub fn merge(&mut self, read: usize, write: usize) -> bool {
        let (read_start, read_end) = self.range(read);
        let (write_start, write_end) = self.range(write);
        let vector = &mut self.vector[..];
        let mut changed = false;
        for (read_index, write_index) in (read_start..read_end).zip(write_start..write_end) {
            let v1 = vector[write_index];
            let v2 = v1 | vector[read_index];
            vector[write_index] = v2;
            changed = changed | (v1 != v2);
        }
        changed
    }

    /// Iterates through all the columns set to true in a given row of
    /// the matrix.
    pub fn iter<'a>(&'a self, row: usize) -> BitVectorIter<'a> {
        let (start, end) = self.range(row);
        BitVectorIter {
            iter: self.vector[start..end].iter(),
            current: 0,
            idx: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SparseBitMatrix<R, C>
where
    R: Idx,
    C: Idx,
{
    vector: IndexVec<R, SparseBitSet<C>>,
}

impl<R: Idx, C: Idx> SparseBitMatrix<R, C> {
    /// Create a new `rows x columns` matrix, initially empty.
    pub fn new(rows: R, _columns: C) -> SparseBitMatrix<R, C> {
        SparseBitMatrix {
            vector: IndexVec::from_elem_n(SparseBitSet::new(), rows.index()),
        }
    }

    /// Sets the cell at `(row, column)` to true. Put another way, insert
    /// `column` to the bitset for `row`.
    ///
    /// Returns true if this changed the matrix, and false otherwise.
    pub fn add(&mut self, row: R, column: C) -> bool {
        self.vector[row].insert(column)
    }

    /// Do the bits from `row` contain `column`? Put another way, is
    /// the matrix cell at `(row, column)` true?  Put yet another way,
    /// if the matrix represents (transitive) reachability, can
    /// `row` reach `column`?
    pub fn contains(&self, row: R, column: C) -> bool {
        self.vector[row].contains(column)
    }

    /// Add the bits from row `read` to the bits from row `write`,
    /// return true if anything changed.
    ///
    /// This is used when computing transitive reachability because if
    /// you have an edge `write -> read`, because in that case
    /// `write` can reach everything that `read` can (and
    /// potentially more).
    pub fn merge(&mut self, read: R, write: R) -> bool {
        let mut changed = false;

        if read != write {
            let (bit_set_read, bit_set_write) = self.vector.pick2_mut(read, write);

            for read_chunk in bit_set_read.chunks() {
                changed = changed | bit_set_write.insert_chunk(read_chunk).any();
            }
        }

        changed
    }

    /// True if `sub` is a subset of `sup`
    pub fn is_subset(&self, sub: R, sup: R) -> bool {
        sub == sup || {
            let bit_set_sub = &self.vector[sub];
            let bit_set_sup = &self.vector[sup];
            bit_set_sub
                .chunks()
                .all(|read_chunk| read_chunk.bits_eq(bit_set_sup.contains_chunk(read_chunk)))
        }
    }

    /// Iterates through all the columns set to true in a given row of
    /// the matrix.
    pub fn iter<'a>(&'a self, row: R) -> impl Iterator<Item = C> + 'a {
        self.vector[row].iter()
    }
}

#[derive(Clone, Debug)]
pub struct SparseBitSet<I: Idx> {
    chunk_bits: BTreeMap<u32, Word>,
    _marker: PhantomData<I>,
}

#[derive(Copy, Clone)]
pub struct SparseChunk<I> {
    key: u32,
    bits: Word,
    _marker: PhantomData<I>,
}

impl<I: Idx> SparseChunk<I> {
    #[inline]
    pub fn one(index: I) -> Self {
        let index = index.index();
        let key_usize = index / 128;
        let key = key_usize as u32;
        assert_eq!(key as usize, key_usize);
        SparseChunk {
            key,
            bits: 1 << (index % 128),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn any(&self) -> bool {
        self.bits != 0
    }

    #[inline]
    pub fn bits_eq(&self, other: SparseChunk<I>) -> bool {
        self.bits == other.bits
    }

    pub fn iter(&self) -> impl Iterator<Item = I> {
        let base = self.key as usize * 128;
        let mut bits = self.bits;
        (0..128)
            .map(move |i| {
                let current_bits = bits;
                bits >>= 1;
                (i, current_bits)
            })
            .take_while(|&(_, bits)| bits != 0)
            .filter_map(move |(i, bits)| {
                if (bits & 1) != 0 {
                    Some(I::new(base + i))
                } else {
                    None
                }
            })
    }
}

impl<I: Idx> SparseBitSet<I> {
    pub fn new() -> Self {
        SparseBitSet {
            chunk_bits: BTreeMap::new(),
            _marker: PhantomData,
        }
    }

    pub fn capacity(&self) -> usize {
        self.chunk_bits.len() * 128
    }

    /// Returns a chunk containing only those bits that are already
    /// present. You can test therefore if `self` contains all the
    /// bits in chunk already by doing `chunk ==
    /// self.contains_chunk(chunk)`.
    pub fn contains_chunk(&self, chunk: SparseChunk<I>) -> SparseChunk<I> {
        SparseChunk {
            bits: self.chunk_bits
                .get(&chunk.key)
                .map_or(0, |bits| bits & chunk.bits),
            ..chunk
        }
    }

    /// Modifies `self` to contain all the bits from `chunk` (in
    /// addition to any pre-existing bits); returns a new chunk that
    /// contains only those bits that were newly added. You can test
    /// if anything was inserted by invoking `any()` on the returned
    /// value.
    pub fn insert_chunk(&mut self, chunk: SparseChunk<I>) -> SparseChunk<I> {
        if chunk.bits == 0 {
            return chunk;
        }
        let bits = self.chunk_bits.entry(chunk.key).or_insert(0);
        let old_bits = *bits;
        let new_bits = old_bits | chunk.bits;
        *bits = new_bits;
        let changed = new_bits ^ old_bits;
        SparseChunk {
            bits: changed,
            ..chunk
        }
    }

    pub fn remove_chunk(&mut self, chunk: SparseChunk<I>) -> SparseChunk<I> {
        if chunk.bits == 0 {
            return chunk;
        }
        let changed = match self.chunk_bits.entry(chunk.key) {
            Entry::Occupied(mut bits) => {
                let old_bits = *bits.get();
                let new_bits = old_bits & !chunk.bits;
                if new_bits == 0 {
                    bits.remove();
                } else {
                    bits.insert(new_bits);
                }
                new_bits ^ old_bits
            }
            Entry::Vacant(_) => 0,
        };
        SparseChunk {
            bits: changed,
            ..chunk
        }
    }

    pub fn clear(&mut self) {
        self.chunk_bits.clear();
    }

    pub fn chunks<'a>(&'a self) -> impl Iterator<Item = SparseChunk<I>> + 'a {
        self.chunk_bits.iter().map(|(&key, &bits)| SparseChunk {
            key,
            bits,
            _marker: PhantomData,
        })
    }

    pub fn contains(&self, index: I) -> bool {
        self.contains_chunk(SparseChunk::one(index)).any()
    }

    pub fn insert(&mut self, index: I) -> bool {
        self.insert_chunk(SparseChunk::one(index)).any()
    }

    pub fn remove(&mut self, index: I) -> bool {
        self.remove_chunk(SparseChunk::one(index)).any()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = I> + 'a {
        self.chunks().flat_map(|chunk| chunk.iter())
    }
}

#[inline]
fn words(elements: usize) -> usize {
    (elements + WORD_BITS - 1) / WORD_BITS
}

#[inline]
fn word_mask(index: usize) -> (usize, Word) {
    let word = index / WORD_BITS;
    let mask = 1 << (index % WORD_BITS);
    (word, mask)
}

#[test]
fn bitvec_iter_works() {
    let mut bitvec = BitVector::new(100);
    bitvec.insert(1);
    bitvec.insert(10);
    bitvec.insert(19);
    bitvec.insert(62);
    bitvec.insert(63);
    bitvec.insert(64);
    bitvec.insert(65);
    bitvec.insert(66);
    bitvec.insert(99);
    assert_eq!(
        bitvec.iter().collect::<Vec<_>>(),
        [1, 10, 19, 62, 63, 64, 65, 66, 99]
    );
}

#[test]
fn bitvec_iter_works_2() {
    let mut bitvec = BitVector::new(319);
    bitvec.insert(0);
    bitvec.insert(127);
    bitvec.insert(191);
    bitvec.insert(255);
    bitvec.insert(319);
    assert_eq!(bitvec.iter().collect::<Vec<_>>(), [0, 127, 191, 255, 319]);
}

#[test]
fn union_two_vecs() {
    let mut vec1 = BitVector::new(65);
    let mut vec2 = BitVector::new(65);
    assert!(vec1.insert(3));
    assert!(!vec1.insert(3));
    assert!(vec2.insert(5));
    assert!(vec2.insert(64));
    assert!(vec1.insert_all(&vec2));
    assert!(!vec1.insert_all(&vec2));
    assert!(vec1.contains(3));
    assert!(!vec1.contains(4));
    assert!(vec1.contains(5));
    assert!(!vec1.contains(63));
    assert!(vec1.contains(64));
}

#[test]
fn grow() {
    let mut vec1 = BitVector::new(65);
    for index in 0..65 {
        assert!(vec1.insert(index));
        assert!(!vec1.insert(index));
    }
    vec1.grow(128);

    // Check if the bits set before growing are still set
    for index in 0..65 {
        assert!(vec1.contains(index));
    }

    // Check if the new bits are all un-set
    for index in 65..128 {
        assert!(!vec1.contains(index));
    }

    // Check that we can set all new bits without running out of bounds
    for index in 65..128 {
        assert!(vec1.insert(index));
        assert!(!vec1.insert(index));
    }
}

#[test]
fn matrix_intersection() {
    let mut vec1 = BitMatrix::new(200, 200);

    // (*) Elements reachable from both 2 and 65.

    vec1.add(2, 3);
    vec1.add(2, 6);
    vec1.add(2, 10); // (*)
    vec1.add(2, 64); // (*)
    vec1.add(2, 65);
    vec1.add(2, 130);
    vec1.add(2, 160); // (*)

    vec1.add(64, 133);

    vec1.add(65, 2);
    vec1.add(65, 8);
    vec1.add(65, 10); // (*)
    vec1.add(65, 64); // (*)
    vec1.add(65, 68);
    vec1.add(65, 133);
    vec1.add(65, 160); // (*)

    let intersection = vec1.intersection(2, 64);
    assert!(intersection.is_empty());

    let intersection = vec1.intersection(2, 65);
    assert_eq!(intersection, &[10, 64, 160]);
}

#[test]
fn matrix_iter() {
    let mut matrix = BitMatrix::new(64, 100);
    matrix.add(3, 22);
    matrix.add(3, 75);
    matrix.add(2, 99);
    matrix.add(4, 0);
    matrix.merge(3, 5);

    let expected = [99];
    let mut iter = expected.iter();
    for i in matrix.iter(2) {
        let j = *iter.next().unwrap();
        assert_eq!(i, j);
    }
    assert!(iter.next().is_none());

    let expected = [22, 75];
    let mut iter = expected.iter();
    for i in matrix.iter(3) {
        let j = *iter.next().unwrap();
        assert_eq!(i, j);
    }
    assert!(iter.next().is_none());

    let expected = [0];
    let mut iter = expected.iter();
    for i in matrix.iter(4) {
        let j = *iter.next().unwrap();
        assert_eq!(i, j);
    }
    assert!(iter.next().is_none());

    let expected = [22, 75];
    let mut iter = expected.iter();
    for i in matrix.iter(5) {
        let j = *iter.next().unwrap();
        assert_eq!(i, j);
    }
    assert!(iter.next().is_none());
}
