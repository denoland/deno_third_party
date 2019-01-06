//! An interpreter for MIR used in CTFE and by miri

#[macro_export]
macro_rules! err {
    ($($tt:tt)*) => { Err($crate::mir::interpret::EvalErrorKind::$($tt)*.into()) };
}

mod error;
mod value;
mod allocation;
mod pointer;

pub use self::error::{
    EvalError, EvalResult, EvalErrorKind, AssertMessage, ConstEvalErr, struct_error,
    FrameInfo, ConstEvalRawResult, ConstEvalResult, ErrorHandled,
};

pub use self::value::{Scalar, ScalarMaybeUndef, RawConst, ConstValue};

pub use self::allocation::{
    InboundsCheck, Allocation, AllocationExtra,
    Relocations, UndefMask,
};

pub use self::pointer::{Pointer, PointerArithmetic};

use std::fmt;
use mir;
use hir::def_id::DefId;
use ty::{self, TyCtxt, Instance};
use ty::layout::{self, Size};
use std::io;
use rustc_serialize::{Encoder, Decodable, Encodable};
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::sync::{Lock as Mutex, HashMapExt};
use rustc_data_structures::tiny_list::TinyList;
use byteorder::{WriteBytesExt, ReadBytesExt, LittleEndian, BigEndian};
use ty::codec::TyDecoder;
use std::sync::atomic::{AtomicU32, Ordering};
use std::num::NonZeroU32;

/// Uniquely identifies a specific constant or static.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, RustcEncodable, RustcDecodable)]
pub struct GlobalId<'tcx> {
    /// For a constant or static, the `Instance` of the item itself.
    /// For a promoted global, the `Instance` of the function they belong to.
    pub instance: ty::Instance<'tcx>,

    /// The index for promoted globals within their function's `Mir`.
    pub promoted: Option<mir::Promoted>,
}

#[derive(Copy, Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct AllocId(pub u64);

impl ::rustc_serialize::UseSpecializedEncodable for AllocId {}
impl ::rustc_serialize::UseSpecializedDecodable for AllocId {}

#[derive(RustcDecodable, RustcEncodable)]
enum AllocDiscriminant {
    Alloc,
    Fn,
    Static,
}

pub fn specialized_encode_alloc_id<
    'a, 'tcx,
    E: Encoder,
>(
    encoder: &mut E,
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    alloc_id: AllocId,
) -> Result<(), E::Error> {
    let alloc_kind: AllocKind<'tcx> =
        tcx.alloc_map.lock().get(alloc_id).expect("no value for AllocId");
    match alloc_kind {
        AllocKind::Memory(alloc) => {
            trace!("encoding {:?} with {:#?}", alloc_id, alloc);
            AllocDiscriminant::Alloc.encode(encoder)?;
            alloc.encode(encoder)?;
        }
        AllocKind::Function(fn_instance) => {
            trace!("encoding {:?} with {:#?}", alloc_id, fn_instance);
            AllocDiscriminant::Fn.encode(encoder)?;
            fn_instance.encode(encoder)?;
        }
        AllocKind::Static(did) => {
            // referring to statics doesn't need to know about their allocations,
            // just about its DefId
            AllocDiscriminant::Static.encode(encoder)?;
            did.encode(encoder)?;
        }
    }
    Ok(())
}

// Used to avoid infinite recursion when decoding cyclic allocations.
type DecodingSessionId = NonZeroU32;

#[derive(Clone)]
enum State {
    Empty,
    InProgressNonAlloc(TinyList<DecodingSessionId>),
    InProgress(TinyList<DecodingSessionId>, AllocId),
    Done(AllocId),
}

pub struct AllocDecodingState {
    // For each AllocId we keep track of which decoding state it's currently in.
    decoding_state: Vec<Mutex<State>>,
    // The offsets of each allocation in the data stream.
    data_offsets: Vec<u32>,
}

impl AllocDecodingState {

    pub fn new_decoding_session(&self) -> AllocDecodingSession<'_> {
        static DECODER_SESSION_ID: AtomicU32 = AtomicU32::new(0);
        let counter = DECODER_SESSION_ID.fetch_add(1, Ordering::SeqCst);

        // Make sure this is never zero
        let session_id = DecodingSessionId::new((counter & 0x7FFFFFFF) + 1).unwrap();

        AllocDecodingSession {
            state: self,
            session_id,
        }
    }

    pub fn new(data_offsets: Vec<u32>) -> AllocDecodingState {
        let decoding_state = vec![Mutex::new(State::Empty); data_offsets.len()];

        AllocDecodingState {
            decoding_state,
            data_offsets,
        }
    }
}

#[derive(Copy, Clone)]
pub struct AllocDecodingSession<'s> {
    state: &'s AllocDecodingState,
    session_id: DecodingSessionId,
}

impl<'s> AllocDecodingSession<'s> {

    // Decodes an AllocId in a thread-safe way.
    pub fn decode_alloc_id<'a, 'tcx, D>(&self,
                                        decoder: &mut D)
                                        -> Result<AllocId, D::Error>
        where D: TyDecoder<'a, 'tcx>,
              'tcx: 'a,
    {
        // Read the index of the allocation
        let idx = decoder.read_u32()? as usize;
        let pos = self.state.data_offsets[idx] as usize;

        // Decode the AllocDiscriminant now so that we know if we have to reserve an
        // AllocId.
        let (alloc_kind, pos) = decoder.with_position(pos, |decoder| {
            let alloc_kind = AllocDiscriminant::decode(decoder)?;
            Ok((alloc_kind, decoder.position()))
        })?;

        // Check the decoding state, see if it's already decoded or if we should
        // decode it here.
        let alloc_id = {
            let mut entry = self.state.decoding_state[idx].lock();

            match *entry {
                State::Done(alloc_id) => {
                    return Ok(alloc_id);
                }
                ref mut entry @ State::Empty => {
                    // We are allowed to decode
                    match alloc_kind {
                        AllocDiscriminant::Alloc => {
                            // If this is an allocation, we need to reserve an
                            // AllocId so we can decode cyclic graphs.
                            let alloc_id = decoder.tcx().alloc_map.lock().reserve();
                            *entry = State::InProgress(
                                TinyList::new_single(self.session_id),
                                alloc_id);
                            Some(alloc_id)
                        },
                        AllocDiscriminant::Fn | AllocDiscriminant::Static => {
                            // Fns and statics cannot be cyclic and their AllocId
                            // is determined later by interning
                            *entry = State::InProgressNonAlloc(
                                TinyList::new_single(self.session_id));
                            None
                        }
                    }
                }
                State::InProgressNonAlloc(ref mut sessions) => {
                    if sessions.contains(&self.session_id) {
                        bug!("This should be unreachable")
                    } else {
                        // Start decoding concurrently
                        sessions.insert(self.session_id);
                        None
                    }
                }
                State::InProgress(ref mut sessions, alloc_id) => {
                    if sessions.contains(&self.session_id) {
                        // Don't recurse.
                        return Ok(alloc_id)
                    } else {
                        // Start decoding concurrently
                        sessions.insert(self.session_id);
                        Some(alloc_id)
                    }
                }
            }
        };

        // Now decode the actual data
        let alloc_id = decoder.with_position(pos, |decoder| {
            match alloc_kind {
                AllocDiscriminant::Alloc => {
                    let allocation = <&'tcx Allocation as Decodable>::decode(decoder)?;
                    // We already have a reserved AllocId.
                    let alloc_id = alloc_id.unwrap();
                    trace!("decoded alloc {:?} {:#?}", alloc_id, allocation);
                    decoder.tcx().alloc_map.lock().set_alloc_id_same_memory(alloc_id, allocation);
                    Ok(alloc_id)
                },
                AllocDiscriminant::Fn => {
                    assert!(alloc_id.is_none());
                    trace!("creating fn alloc id");
                    let instance = ty::Instance::decode(decoder)?;
                    trace!("decoded fn alloc instance: {:?}", instance);
                    let alloc_id = decoder.tcx().alloc_map.lock().create_fn_alloc(instance);
                    Ok(alloc_id)
                },
                AllocDiscriminant::Static => {
                    assert!(alloc_id.is_none());
                    trace!("creating extern static alloc id at");
                    let did = DefId::decode(decoder)?;
                    let alloc_id = decoder.tcx().alloc_map.lock().intern_static(did);
                    Ok(alloc_id)
                }
            }
        })?;

        self.state.decoding_state[idx].with_lock(|entry| {
            *entry = State::Done(alloc_id);
        });

        Ok(alloc_id)
    }
}

impl fmt::Display for AllocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, RustcDecodable, RustcEncodable)]
pub enum AllocKind<'tcx> {
    /// The alloc id is used as a function pointer
    Function(Instance<'tcx>),
    /// The alloc id points to a "lazy" static variable that did not get computed (yet).
    /// This is also used to break the cycle in recursive statics.
    Static(DefId),
    /// The alloc id points to memory
    Memory(&'tcx Allocation),
}

pub struct AllocMap<'tcx> {
    /// Lets you know what an AllocId refers to
    id_to_kind: FxHashMap<AllocId, AllocKind<'tcx>>,

    /// Used to ensure that statics only get one associated AllocId
    type_interner: FxHashMap<AllocKind<'tcx>, AllocId>,

    /// The AllocId to assign to the next requested id.
    /// Always incremented, never gets smaller.
    next_id: AllocId,
}

impl<'tcx> AllocMap<'tcx> {
    pub fn new() -> Self {
        AllocMap {
            id_to_kind: Default::default(),
            type_interner: Default::default(),
            next_id: AllocId(0),
        }
    }

    /// Obtains a new allocation ID that can be referenced but does not
    /// yet have an allocation backing it.
    ///
    /// Make sure to call `set_alloc_id_memory` or `set_alloc_id_same_memory` before returning such
    /// an `AllocId` from a query.
    pub fn reserve(
        &mut self,
    ) -> AllocId {
        let next = self.next_id;
        self.next_id.0 = self.next_id.0
            .checked_add(1)
            .expect("You overflowed a u64 by incrementing by 1... \
                     You've just earned yourself a free drink if we ever meet. \
                     Seriously, how did you do that?!");
        next
    }

    fn intern(&mut self, alloc_kind: AllocKind<'tcx>) -> AllocId {
        if let Some(&alloc_id) = self.type_interner.get(&alloc_kind) {
            return alloc_id;
        }
        let id = self.reserve();
        debug!("creating alloc_kind {:?} with id {}", alloc_kind, id);
        self.id_to_kind.insert(id, alloc_kind.clone());
        self.type_interner.insert(alloc_kind, id);
        id
    }

    /// Functions cannot be identified by pointers, as asm-equal functions can get deduplicated
    /// by the linker and functions can be duplicated across crates.
    /// We thus generate a new `AllocId` for every mention of a function. This means that
    /// `main as fn() == main as fn()` is false, while `let x = main as fn(); x == x` is true.
    pub fn create_fn_alloc(&mut self, instance: Instance<'tcx>) -> AllocId {
        let id = self.reserve();
        self.id_to_kind.insert(id, AllocKind::Function(instance));
        id
    }

    /// Returns `None` in case the `AllocId` is dangling. An `EvalContext` can still have a
    /// local `Allocation` for that `AllocId`, but having such an `AllocId` in a constant is
    /// illegal and will likely ICE.
    /// This function exists to allow const eval to detect the difference between evaluation-
    /// local dangling pointers and allocations in constants/statics.
    pub fn get(&self, id: AllocId) -> Option<AllocKind<'tcx>> {
        self.id_to_kind.get(&id).cloned()
    }

    /// Panics if the `AllocId` does not refer to an `Allocation`
    pub fn unwrap_memory(&self, id: AllocId) -> &'tcx Allocation {
        match self.get(id) {
            Some(AllocKind::Memory(mem)) => mem,
            _ => bug!("expected allocation id {} to point to memory", id),
        }
    }

    /// Generate an `AllocId` for a static or return a cached one in case this function has been
    /// called on the same static before.
    pub fn intern_static(&mut self, static_id: DefId) -> AllocId {
        self.intern(AllocKind::Static(static_id))
    }

    /// Intern the `Allocation` and return a new `AllocId`, even if there's already an identical
    /// `Allocation` with a different `AllocId`.
    // FIXME: is this really necessary? Can we ensure `FOO` and `BAR` being different after codegen
    // in `static FOO: u32 = 42; static BAR: u32 = 42;` even if they reuse the same allocation
    // inside rustc?
    pub fn allocate(&mut self, mem: &'tcx Allocation) -> AllocId {
        let id = self.reserve();
        self.set_alloc_id_memory(id, mem);
        id
    }

    /// Freeze an `AllocId` created with `reserve` by pointing it at an `Allocation`. Trying to
    /// call this function twice, even with the same `Allocation` will ICE the compiler.
    pub fn set_alloc_id_memory(&mut self, id: AllocId, mem: &'tcx Allocation) {
        if let Some(old) = self.id_to_kind.insert(id, AllocKind::Memory(mem)) {
            bug!("tried to set allocation id {}, but it was already existing as {:#?}", id, old);
        }
    }

    /// Freeze an `AllocId` created with `reserve` by pointing it at an `Allocation`. May be called
    /// twice for the same `(AllocId, Allocation)` pair.
    fn set_alloc_id_same_memory(&mut self, id: AllocId, mem: &'tcx Allocation) {
        self.id_to_kind.insert_same(id, AllocKind::Memory(mem));
    }
}

////////////////////////////////////////////////////////////////////////////////
// Methods to access integers in the target endianness
////////////////////////////////////////////////////////////////////////////////

pub fn write_target_uint(
    endianness: layout::Endian,
    mut target: &mut [u8],
    data: u128,
) -> Result<(), io::Error> {
    let len = target.len();
    match endianness {
        layout::Endian::Little => target.write_uint128::<LittleEndian>(data, len),
        layout::Endian::Big => target.write_uint128::<BigEndian>(data, len),
    }
}

pub fn read_target_uint(endianness: layout::Endian, mut source: &[u8]) -> Result<u128, io::Error> {
    match endianness {
        layout::Endian::Little => source.read_uint128::<LittleEndian>(source.len()),
        layout::Endian::Big => source.read_uint128::<BigEndian>(source.len()),
    }
}

////////////////////////////////////////////////////////////////////////////////
// Methods to facilitate working with signed integers stored in a u128
////////////////////////////////////////////////////////////////////////////////

pub fn sign_extend(value: u128, size: Size) -> u128 {
    let size = size.bits();
    // sign extend
    let shift = 128 - size;
    // shift the unsigned value to the left
    // and back to the right as signed (essentially fills with FF on the left)
    (((value << shift) as i128) >> shift) as u128
}

pub fn truncate(value: u128, size: Size) -> u128 {
    let size = size.bits();
    let shift = 128 - size;
    // truncate (shift left to drop out leftover values, shift right to fill with zeroes)
    (value << shift) >> shift
}
