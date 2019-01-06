use fmt;
use marker;
use usize;

use super::{FusedIterator, TrustedLen};

/// An iterator that repeats an element endlessly.
///
/// This `struct` is created by the [`repeat`] function. See its documentation for more.
///
/// [`repeat`]: fn.repeat.html
#[derive(Clone, Debug)]
#[stable(feature = "rust1", since = "1.0.0")]
pub struct Repeat<A> {
    element: A
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<A: Clone> Iterator for Repeat<A> {
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> { Some(self.element.clone()) }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { (usize::MAX, None) }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<A: Clone> DoubleEndedIterator for Repeat<A> {
    #[inline]
    fn next_back(&mut self) -> Option<A> { Some(self.element.clone()) }
}

#[stable(feature = "fused", since = "1.26.0")]
impl<A: Clone> FusedIterator for Repeat<A> {}

#[unstable(feature = "trusted_len", issue = "37572")]
unsafe impl<A: Clone> TrustedLen for Repeat<A> {}

/// Creates a new iterator that endlessly repeats a single element.
///
/// The `repeat()` function repeats a single value over and over and over and
/// over and over and 🔁.
///
/// Infinite iterators like `repeat()` are often used with adapters like
/// [`take`], in order to make them finite.
///
/// [`take`]: trait.Iterator.html#method.take
///
/// If the element type of the iterator you need does not implement `Clone`,
/// or if you do not want to keep the repeated element in memory, you can
/// instead use the [`repeat_with`] function.
///
/// [`repeat_with`]: fn.repeat_with.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::iter;
///
/// // the number four 4ever:
/// let mut fours = iter::repeat(4);
///
/// assert_eq!(Some(4), fours.next());
/// assert_eq!(Some(4), fours.next());
/// assert_eq!(Some(4), fours.next());
/// assert_eq!(Some(4), fours.next());
/// assert_eq!(Some(4), fours.next());
///
/// // yup, still four
/// assert_eq!(Some(4), fours.next());
/// ```
///
/// Going finite with [`take`]:
///
/// ```
/// use std::iter;
///
/// // that last example was too many fours. Let's only have four fours.
/// let mut four_fours = iter::repeat(4).take(4);
///
/// assert_eq!(Some(4), four_fours.next());
/// assert_eq!(Some(4), four_fours.next());
/// assert_eq!(Some(4), four_fours.next());
/// assert_eq!(Some(4), four_fours.next());
///
/// // ... and now we're done
/// assert_eq!(None, four_fours.next());
/// ```
#[inline]
#[stable(feature = "rust1", since = "1.0.0")]
pub fn repeat<T: Clone>(elt: T) -> Repeat<T> {
    Repeat{element: elt}
}

/// An iterator that repeats elements of type `A` endlessly by
/// applying the provided closure `F: FnMut() -> A`.
///
/// This `struct` is created by the [`repeat_with`] function.
/// See its documentation for more.
///
/// [`repeat_with`]: fn.repeat_with.html
#[derive(Copy, Clone, Debug)]
#[stable(feature = "iterator_repeat_with", since = "1.28.0")]
pub struct RepeatWith<F> {
    repeater: F
}

#[stable(feature = "iterator_repeat_with", since = "1.28.0")]
impl<A, F: FnMut() -> A> Iterator for RepeatWith<F> {
    type Item = A;

    #[inline]
    fn next(&mut self) -> Option<A> { Some((self.repeater)()) }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { (usize::MAX, None) }
}

#[stable(feature = "iterator_repeat_with", since = "1.28.0")]
impl<A, F: FnMut() -> A> FusedIterator for RepeatWith<F> {}

#[unstable(feature = "trusted_len", issue = "37572")]
unsafe impl<A, F: FnMut() -> A> TrustedLen for RepeatWith<F> {}

/// Creates a new iterator that repeats elements of type `A` endlessly by
/// applying the provided closure, the repeater, `F: FnMut() -> A`.
///
/// The `repeat_with()` function calls the repeater over and over and over and
/// over and over and 🔁.
///
/// Infinite iterators like `repeat_with()` are often used with adapters like
/// [`take`], in order to make them finite.
///
/// [`take`]: trait.Iterator.html#method.take
///
/// If the element type of the iterator you need implements `Clone`, and
/// it is OK to keep the source element in memory, you should instead use
/// the [`repeat`] function.
///
/// [`repeat`]: fn.repeat.html
///
/// An iterator produced by `repeat_with()` is not a `DoubleEndedIterator`.
/// If you need `repeat_with()` to return a `DoubleEndedIterator`,
/// please open a GitHub issue explaining your use case.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::iter;
///
/// // let's assume we have some value of a type that is not `Clone`
/// // or which don't want to have in memory just yet because it is expensive:
/// #[derive(PartialEq, Debug)]
/// struct Expensive;
///
/// // a particular value forever:
/// let mut things = iter::repeat_with(|| Expensive);
///
/// assert_eq!(Some(Expensive), things.next());
/// assert_eq!(Some(Expensive), things.next());
/// assert_eq!(Some(Expensive), things.next());
/// assert_eq!(Some(Expensive), things.next());
/// assert_eq!(Some(Expensive), things.next());
/// ```
///
/// Using mutation and going finite:
///
/// ```rust
/// use std::iter;
///
/// // From the zeroth to the third power of two:
/// let mut curr = 1;
/// let mut pow2 = iter::repeat_with(|| { let tmp = curr; curr *= 2; tmp })
///                     .take(4);
///
/// assert_eq!(Some(1), pow2.next());
/// assert_eq!(Some(2), pow2.next());
/// assert_eq!(Some(4), pow2.next());
/// assert_eq!(Some(8), pow2.next());
///
/// // ... and now we're done
/// assert_eq!(None, pow2.next());
/// ```
#[inline]
#[stable(feature = "iterator_repeat_with", since = "1.28.0")]
pub fn repeat_with<A, F: FnMut() -> A>(repeater: F) -> RepeatWith<F> {
    RepeatWith { repeater }
}

/// An iterator that yields nothing.
///
/// This `struct` is created by the [`empty`] function. See its documentation for more.
///
/// [`empty`]: fn.empty.html
#[stable(feature = "iter_empty", since = "1.2.0")]
pub struct Empty<T>(marker::PhantomData<T>);

#[stable(feature = "core_impl_debug", since = "1.9.0")]
impl<T> fmt::Debug for Empty<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad("Empty")
    }
}

#[stable(feature = "iter_empty", since = "1.2.0")]
impl<T> Iterator for Empty<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>){
        (0, Some(0))
    }
}

#[stable(feature = "iter_empty", since = "1.2.0")]
impl<T> DoubleEndedIterator for Empty<T> {
    fn next_back(&mut self) -> Option<T> {
        None
    }
}

#[stable(feature = "iter_empty", since = "1.2.0")]
impl<T> ExactSizeIterator for Empty<T> {
    fn len(&self) -> usize {
        0
    }
}

#[unstable(feature = "trusted_len", issue = "37572")]
unsafe impl<T> TrustedLen for Empty<T> {}

#[stable(feature = "fused", since = "1.26.0")]
impl<T> FusedIterator for Empty<T> {}

// not #[derive] because that adds a Clone bound on T,
// which isn't necessary.
#[stable(feature = "iter_empty", since = "1.2.0")]
impl<T> Clone for Empty<T> {
    fn clone(&self) -> Empty<T> {
        Empty(marker::PhantomData)
    }
}

// not #[derive] because that adds a Default bound on T,
// which isn't necessary.
#[stable(feature = "iter_empty", since = "1.2.0")]
impl<T> Default for Empty<T> {
    fn default() -> Empty<T> {
        Empty(marker::PhantomData)
    }
}

/// Creates an iterator that yields nothing.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::iter;
///
/// // this could have been an iterator over i32, but alas, it's just not.
/// let mut nope = iter::empty::<i32>();
///
/// assert_eq!(None, nope.next());
/// ```
#[stable(feature = "iter_empty", since = "1.2.0")]
pub const fn empty<T>() -> Empty<T> {
    Empty(marker::PhantomData)
}

/// An iterator that yields an element exactly once.
///
/// This `struct` is created by the [`once`] function. See its documentation for more.
///
/// [`once`]: fn.once.html
#[derive(Clone, Debug)]
#[stable(feature = "iter_once", since = "1.2.0")]
pub struct Once<T> {
    inner: ::option::IntoIter<T>
}

#[stable(feature = "iter_once", since = "1.2.0")]
impl<T> Iterator for Once<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[stable(feature = "iter_once", since = "1.2.0")]
impl<T> DoubleEndedIterator for Once<T> {
    fn next_back(&mut self) -> Option<T> {
        self.inner.next_back()
    }
}

#[stable(feature = "iter_once", since = "1.2.0")]
impl<T> ExactSizeIterator for Once<T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[unstable(feature = "trusted_len", issue = "37572")]
unsafe impl<T> TrustedLen for Once<T> {}

#[stable(feature = "fused", since = "1.26.0")]
impl<T> FusedIterator for Once<T> {}

/// Creates an iterator that yields an element exactly once.
///
/// This is commonly used to adapt a single value into a [`chain`] of other
/// kinds of iteration. Maybe you have an iterator that covers almost
/// everything, but you need an extra special case. Maybe you have a function
/// which works on iterators, but you only need to process one value.
///
/// [`chain`]: trait.Iterator.html#method.chain
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::iter;
///
/// // one is the loneliest number
/// let mut one = iter::once(1);
///
/// assert_eq!(Some(1), one.next());
///
/// // just one, that's all we get
/// assert_eq!(None, one.next());
/// ```
///
/// Chaining together with another iterator. Let's say that we want to iterate
/// over each file of the `.foo` directory, but also a configuration file,
/// `.foorc`:
///
/// ```no_run
/// use std::iter;
/// use std::fs;
/// use std::path::PathBuf;
///
/// let dirs = fs::read_dir(".foo").unwrap();
///
/// // we need to convert from an iterator of DirEntry-s to an iterator of
/// // PathBufs, so we use map
/// let dirs = dirs.map(|file| file.unwrap().path());
///
/// // now, our iterator just for our config file
/// let config = iter::once(PathBuf::from(".foorc"));
///
/// // chain the two iterators together into one big iterator
/// let files = dirs.chain(config);
///
/// // this will give us all of the files in .foo as well as .foorc
/// for f in files {
///     println!("{:?}", f);
/// }
/// ```
#[stable(feature = "iter_once", since = "1.2.0")]
pub fn once<T>(value: T) -> Once<T> {
    Once { inner: Some(value).into_iter() }
}

/// Creates a new iterator where each iteration calls the provided closure
/// `F: FnMut(&mut St) -> Option<T>`.
///
/// This allows creating a custom iterator with any behavior
/// without using the more verbose syntax of creating a dedicated type
/// and implementing the `Iterator` trait for it.
///
/// In addition to its captures and environment,
/// the closure is given a mutable reference to some state
/// that is preserved across iterations.
/// That state starts as the given `initial_state` value.
///
/// Note that the `Unfold` iterator doesn’t make assumptions about the behavior of the closure,
/// and therefore conservatively does not implement [`FusedIterator`],
/// or override [`Iterator::size_hint`] from its default `(0, None)`.
///
/// [`FusedIterator`]: trait.FusedIterator.html
/// [`Iterator::size_hint`]: trait.Iterator.html#method.size_hint
///
/// # Examples
///
/// Let’s re-implement the counter iterator from [module-level documentation]:
///
/// [module-level documentation]: index.html
///
/// ```
/// #![feature(iter_unfold)]
/// let counter = std::iter::unfold(0, |count| {
///     // Increment our count. This is why we started at zero.
///     *count += 1;
///
///     // Check to see if we've finished counting or not.
///     if *count < 6 {
///         Some(*count)
///     } else {
///         None
///     }
/// });
/// assert_eq!(counter.collect::<Vec<_>>(), &[1, 2, 3, 4, 5]);
/// ```
#[inline]
#[unstable(feature = "iter_unfold", issue = "55977")]
pub fn unfold<St, T, F>(initial_state: St, f: F) -> Unfold<St, F>
    where F: FnMut(&mut St) -> Option<T>
{
    Unfold {
        state: initial_state,
        f,
    }
}

/// An iterator where each iteration calls the provided closure `F: FnMut(&mut St) -> Option<T>`.
///
/// This `struct` is created by the [`unfold`] function.
/// See its documentation for more.
///
/// [`unfold`]: fn.unfold.html
#[derive(Clone)]
#[unstable(feature = "iter_unfold", issue = "55977")]
pub struct Unfold<St, F> {
    state: St,
    f: F,
}

#[unstable(feature = "iter_unfold", issue = "55977")]
impl<St, T, F> Iterator for Unfold<St, F>
    where F: FnMut(&mut St) -> Option<T>
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        (self.f)(&mut self.state)
    }
}

#[unstable(feature = "iter_unfold", issue = "55977")]
impl<St: fmt::Debug, F> fmt::Debug for Unfold<St, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Unfold")
            .field("state", &self.state)
            .finish()
    }
}

/// Creates a new iterator where each successive item is computed based on the preceding one.
///
/// The iterator starts with the given first item (if any)
/// and calls the given `FnMut(&T) -> Option<T>` closure to compute each item’s successor.
///
/// ```
/// #![feature(iter_unfold)]
/// use std::iter::successors;
///
/// let powers_of_10 = successors(Some(1_u16), |n| n.checked_mul(10));
/// assert_eq!(powers_of_10.collect::<Vec<_>>(), &[1, 10, 100, 1_000, 10_000]);
/// ```
#[unstable(feature = "iter_unfold", issue = "55977")]
pub fn successors<T, F>(first: Option<T>, succ: F) -> Successors<T, F>
    where F: FnMut(&T) -> Option<T>
{
    // If this function returned `impl Iterator<Item=T>`
    // it could be based on `unfold` and not need a dedicated type.
    // However having a named `Successors<T, F>` type allows it to be `Clone` when `T` and `F` are.
    Successors {
        next: first,
        succ,
    }
}

/// An new iterator where each successive item is computed based on the preceding one.
///
/// This `struct` is created by the [`successors`] function.
/// See its documentation for more.
///
/// [`successors`]: fn.successors.html
#[derive(Clone)]
#[unstable(feature = "iter_unfold", issue = "55977")]
pub struct Successors<T, F> {
    next: Option<T>,
    succ: F,
}

#[unstable(feature = "iter_unfold", issue = "55977")]
impl<T, F> Iterator for Successors<T, F>
    where F: FnMut(&T) -> Option<T>
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|item| {
            self.next = (self.succ)(&item);
            item
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.next.is_some() {
            (1, None)
        } else {
            (0, Some(0))
        }
    }
}

#[unstable(feature = "iter_unfold", issue = "55977")]
impl<T, F> FusedIterator for Successors<T, F>
    where F: FnMut(&T) -> Option<T>
{}

#[unstable(feature = "iter_unfold", issue = "55977")]
impl<T: fmt::Debug, F> fmt::Debug for Successors<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Successors")
            .field("next", &self.next)
            .finish()
    }
}
