use std::marker::PhantomData;

// enum causes compile error on type mismatch, whereas newtype () would not.
pub enum VectorOffset {}
pub enum StringOffset {}
pub enum ByteStringOffset {}
pub enum UnionOffset {}
pub enum TableOffset {}
pub struct UnionMarker;

pub trait GeneratedStruct {}

pub trait EndianScalar: Sized + PartialEq + Copy + Clone {
    fn to_little_endian(self) -> Self;
    fn from_little_endian(self) -> Self;
}
impl EndianScalar for bool {
    fn to_little_endian(self) -> Self {
        self
    }
    fn from_little_endian(self) -> Self {
        self
    }
}
impl EndianScalar for u8 {
    fn to_little_endian(self) -> Self {
        self
    }
    fn from_little_endian(self) -> Self {
        self
    }
}
impl EndianScalar for i8 {
    fn to_little_endian(self) -> Self {
        Self::to_le(self)
    }
    fn from_little_endian(self) -> Self {
        Self::from_le(self)
    }
}
impl EndianScalar for u16 {
    fn to_little_endian(self) -> Self {
        Self::to_le(self)
    }
    fn from_little_endian(self) -> Self {
        Self::from_le(self)
    }
}
impl EndianScalar for i16 {
    fn to_little_endian(self) -> Self {
        Self::to_le(self)
    }
    fn from_little_endian(self) -> Self {
        Self::from_le(self)
    }
}
impl EndianScalar for u32 {
    fn to_little_endian(self) -> Self {
        Self::to_le(self)
    }
    fn from_little_endian(self) -> Self {
        Self::from_le(self)
    }
}
impl EndianScalar for i32 {
    fn to_little_endian(self) -> Self {
        Self::to_le(self)
    }
    fn from_little_endian(self) -> Self {
        Self::from_le(self)
    }
}
impl EndianScalar for u64 {
    fn to_little_endian(self) -> Self {
        Self::to_le(self)
    }
    fn from_little_endian(self) -> Self {
        Self::from_le(self)
    }
}
impl EndianScalar for i64 {
    fn to_little_endian(self) -> Self {
        Self::to_le(self)
    }
    fn from_little_endian(self) -> Self {
        Self::from_le(self)
    }
}
impl EndianScalar for f32 {
    fn to_little_endian(self) -> Self {
        self
    }
    fn from_little_endian(self) -> Self {
        self
    }
}
impl EndianScalar for f64 {
    fn to_little_endian(self) -> Self {
        self
    }
    fn from_little_endian(self) -> Self {
        self
    }
}

pub const FLATBUFFERS_MAX_BUFFER_SIZE: usize = ((1u64 << 32) - 1) as usize;

pub const FILE_IDENTIFIER_LENGTH: usize = 4;

pub const VTABLE_METADATA_FIELDS: usize = 2;

pub const SIZE_U8: usize = 1;
pub const SIZE_I8: usize = 1;

pub const SIZE_U16: usize = 2;
pub const SIZE_I16: usize = 2;

pub const SIZE_U32: usize = 4;
pub const SIZE_I32: usize = 4;

pub const SIZE_U64: usize = 8;
pub const SIZE_I64: usize = 8;

pub const SIZE_F32: usize = 4;
pub const SIZE_F64: usize = 8;

pub const SIZE_UOFFSET: usize = SIZE_U32;
pub const SIZE_SOFFSET: usize = SIZE_I32;
pub const SIZE_VOFFSET: usize = SIZE_I16;

pub const SIZE_SIZEPREFIX: usize = SIZE_U32;

pub type SOffsetT = i32;
pub type UOffsetT = u32;
pub type VOffsetT = i16;

pub type HeadUOffsetT = UOffsetT;
pub type TailUOffsetT = UOffsetT;

#[derive(Clone, Copy, Debug)]
struct FieldLoc {
    off: UOffsetT,
    id: VOffsetT,
}

#[inline]
pub fn padding_bytes(buf_size: usize, scalar_size: usize) -> usize {
    // ((!buf_size) + 1) & (scalar_size - 1)
    (!buf_size).wrapping_add(1) & (scalar_size.wrapping_sub(1))
}
pub fn field_index_to_field_offset(field_id: VOffsetT) -> VOffsetT {
    // Should correspond to what end_table() below builds up.
    let fixed_fields = 2; // Vtable size and Object Size.
    ((field_id + fixed_fields) * (SIZE_VOFFSET as VOffsetT)) as VOffsetT
}
pub fn field_offset_to_field_index(field_o: VOffsetT) -> VOffsetT {
    debug_assert!(field_o >= 2);
    let fixed_fields = 2; // VTable size and Object Size.
    (field_o / (SIZE_VOFFSET as VOffsetT)) - fixed_fields
}
pub fn to_bytes<'a, T: 'a + Sized>(t: &'a T) -> &'a [u8] {
    let sz = std::mem::size_of::<T>();
    unsafe { std::slice::from_raw_parts((t as *const T) as *const u8, sz) }
}
pub fn emplace_scalar<T: EndianScalar>(s: &mut [u8], x: T) {
    let sz = std::mem::size_of::<T>();
    debug_assert!(s.len() >= sz);

    let mut_ptr = s.as_mut_ptr() as *mut T;
    let val = x.to_little_endian();
    unsafe {
        *mut_ptr = val;
    }
}
pub fn read_scalar_at<T: EndianScalar>(s: &[u8], loc: usize) -> T {
    let buf = &s[loc..loc + std::mem::size_of::<T>()];
    read_scalar(buf)
}
pub fn read_scalar<T: EndianScalar>(s: &[u8]) -> T {
    let sz = std::mem::size_of::<T>();
    debug_assert!(s.len() >= sz);

    let p = s.as_ptr() as *const T;
    let x = unsafe { *p };

    // TODO(rw): is this clone necessary?
    let x = x.clone();

    x.from_little_endian()
}

pub struct FlatBufferBuilder<'fbb> {
    pub owned_buf: Vec<u8>,
    pub cur_idx: usize,

    field_locs: Vec<FieldLoc>,
    written_vtable_revpos: Vec<UOffsetT>,

    nested: bool,
    finished: bool,

    min_align: usize,
    max_voffset: VOffsetT,

    _phantom: PhantomData<&'fbb ()>,
}
impl<'fbb> FlatBufferBuilder<'fbb> {
    pub fn new() -> Self {
        Self::new_with_capacity(0)
    }
    pub fn new_with_capacity(size: usize) -> Self {
        FlatBufferBuilder {
            owned_buf: vec![0u8; size],
            cur_idx: size,

            field_locs: Vec::new(),
            written_vtable_revpos: Vec::new(),

            nested: false,
            finished: false,

            min_align: 0,
            max_voffset: 0,

            _phantom: PhantomData,
        }
    }

    pub fn reset(&mut self) {
        self.owned_buf.clear();
	let cap = self.owned_buf.capacity();
        unsafe {
	    self.owned_buf.set_len(cap);
	}
        self.cur_idx = self.owned_buf.len();

        self.written_vtable_revpos.clear();

        self.nested = false;
        self.finished = false;

        self.min_align = 0;
        self.max_voffset = 0;
    }

    pub fn num_written_vtables(&self) -> usize {
        self.written_vtable_revpos.len()
    }

    fn track_field(&mut self, slot_off: VOffsetT, off: UOffsetT) {
        let fl = FieldLoc {
            id: slot_off,
            off: off,
        };
        self.field_locs.push(fl);
        self.max_voffset = std::cmp::max(self.max_voffset, slot_off);
    }
    pub fn start_table(&mut self, num_fields: VOffsetT) -> Offset<TableOffset> {
        self.assert_not_nested();
        self.nested = true;

        self.field_locs.clear();

        Offset::new(self.get_size() as UOffsetT)
    }
    pub fn get_active_buf_slice<'a>(&'a self) -> &'a [u8] {
        &self.owned_buf[self.cur_idx..]
    }
    fn grow_owned_buf(&mut self) {
        let starting_active_size = self.get_size();

        let old_len = self.owned_buf.len();
        let new_len = std::cmp::max(1, old_len * 2);

        assert!(
            new_len <= FLATBUFFERS_MAX_BUFFER_SIZE,
            "cannot grow buffer beyond 2 gigabytes"
        );

        let diff = new_len - old_len;
        self.owned_buf.resize(new_len, 0);
        self.cur_idx += diff;

        let ending_active_size = self.get_size();
        assert_eq!(starting_active_size, ending_active_size);

        if new_len == 1 {
            return;
        }

        // calculate the midpoint, and safely copy the old end data to the new
        // end position:
        let middle = new_len / 2;
        {
            let (left, right) = &mut self.owned_buf[..].split_at_mut(middle);
            right.copy_from_slice(left);
        }
        // then, zero out the old end data (just to be safe).
        // this should be vectorized by rustc. rust has no stdlib memset.
        for x in &mut self.owned_buf[..middle] {
            *x = 0;
        }
    }
    fn assert_nested(&self, msg: &'static str) {
        assert!(self.nested, msg);
        // we don't assert that self.field_locs.len() >0 because the vtable
        // could be empty (e.g. for empty tables, or for all-default values).
    }
    fn assert_not_nested(&self) {
        assert!(!self.nested);
        assert_eq!(self.field_locs.len(), 0);
    }
    fn assert_finished(&self) {
        assert!(self.finished);
    }
    fn assert_not_finished(&self) {
        assert!(!self.finished);
    }
    pub fn start_vector(&mut self, len: usize, elem_size: usize) -> UOffsetT {
        self.assert_not_nested();
        self.nested = true;
        self.pre_align(len * elem_size, SIZE_UOFFSET);
        self.pre_align(len * elem_size, elem_size); // Just in case elemsize > uoffset_t.
        self.rev_cur_idx()
    }
    pub fn flip_forwards(&self, x: UOffsetT) -> UOffsetT {
        self.get_size() as UOffsetT - x
    }
    // Offset relative to the end of the buffer.
    pub fn rev_cur_idx(&self) -> UOffsetT {
        (self.owned_buf.len() - self.cur_idx) as UOffsetT
    }
    pub fn end_vector<T: 'fbb>(&mut self, num_elems: usize) -> Offset<Vector<'fbb, T>> {
        self.assert_nested("end_vector must be called after a call to start_vector");
        self.nested = false;
        let off = self.push_element_scalar::<UOffsetT>(num_elems as UOffsetT);
        Offset::new(off)
    }
    fn pre_align(&mut self, len: usize, alignment: usize) {
        self.track_min_align(alignment);
        let s = self.get_size() as usize;
        self.fill(padding_bytes(s + len, alignment));
    }
    pub fn get_size(&self) -> usize {
        self.owned_buf.len() - self.cur_idx as usize
    }
    fn fill_big(&mut self, zero_pad_bytes: usize) {
        self.fill(zero_pad_bytes);
    }
    fn fill(&mut self, zero_pad_bytes: usize) {
        self.make_space(zero_pad_bytes);
    }
    fn track_min_align(&mut self, alignment: usize) {
        self.min_align = std::cmp::max(self.min_align, alignment);
    }
    // utf-8 string creation
    pub fn create_string(&mut self, s: &str) -> Offset<&'fbb str> {
        Offset::<&str>::new(self.create_byte_string(s.as_bytes()).value())
    }
    pub fn create_byte_string(&mut self, data: &[u8]) -> Offset<&'fbb [u8]> {
        self.assert_not_nested();
        self.pre_align(data.len() + 1, SIZE_UOFFSET); // Always 0-terminated.
        self.fill(1);
        self.push_bytes(data);
        self.push_element_scalar::<UOffsetT>(data.len() as UOffsetT);
        Offset::new(self.get_size() as UOffsetT)
    }
    pub fn create_byte_vector<'a, 'b>(&'a mut self, data: &'b [u8]) -> Offset<Vector<'fbb, u8>> {
        self.assert_not_nested();
        self.pre_align(data.len(), SIZE_UOFFSET);
        self.push_bytes(data);
        self.push_element_scalar::<UOffsetT>(data.len() as UOffsetT);
        Offset::new(self.get_size() as UOffsetT)
    }
    pub fn create_vector_of_strings<'a, 'b>(
        &'a mut self,
        xs: &'b [&'b str],
    ) -> Offset<Vector<'fbb, ForwardsUOffset<&'fbb str>>> {
        // TODO(rw): write these in-place, then swap their order, to avoid a
        // heap allocation.
        let offsets: Vec<Offset<&str>> = xs
            .iter()
            .rev()
            .map(|s| self.create_string(s))
            .rev()
            .collect();
        self.create_vector_of_reverse_offsets(&offsets[..])
    }
    pub fn create_vector_of_reverse_offsets<'a, 'b, 'c, T: 'fbb>(
        &'a mut self,
        items: &'b [Offset<T>],
    ) -> Offset<Vector<'fbb, ForwardsUOffset<T>>> {
        let elemsize = std::mem::size_of::<Offset<T>>();
        self.start_vector(elemsize, items.len());
        for o in items.iter().rev() {
            self.push_element_scalar_indirect_uoffset(o.value());
        }
        Offset::new(
            self.end_vector::<Offset<Vector<'fbb, ForwardsUOffset<T>>>>(items.len())
                .value(),
        )
    }
    pub fn create_vector_of_scalars<T: EndianScalar + 'fbb>(
        &mut self,
        items: &[T],
    ) -> Offset<Vector<'fbb, T>> {
        // TODO(rw): if host is little-endian, just do a memcpy
        let elemsize = std::mem::size_of::<T>();
        self.start_vector(elemsize, items.len());
        for x in items.iter().rev() {
            self.push_element_scalar(*x);
        }
        Offset::new(self.end_vector::<T>(items.len()).value())
    }
    pub fn create_vector_of_structs<T>(&mut self, items: &[T]) -> Offset<Vector<'fbb, T>> {
        // TODO(rw): just do a memcpy
        let elemsize = std::mem::size_of::<T>();
        self.start_vector(elemsize, items.len());
        for i in (0..items.len()).rev() {
            self.push_bytes(to_bytes(&items[i]));
        }
        Offset::new(self.end_vector::<T>(items.len()).value())
    }
    pub fn end_table(&mut self, off: Offset<TableOffset>) -> Offset<TableOffset> {
        self.assert_nested("end_table must be called after a call to start_table");
        let n = self.write_vtable(off.value());
        self.nested = false;
        self.field_locs.clear();
        self.max_voffset = 0;
        let o = Offset::new(n);
        o
    }
    fn write_vtable(&mut self, table_tail_revloc: UOffsetT) -> UOffsetT {
        // If you get this assert, a corresponding start_table wasn't called.
        self.assert_nested("write_vtable must be called after a call to start_table");

        // Write the vtable offset, which is the start of any Table.
        // We fill its value later.
        //let object_vtable_revloc: UOffsetT = self.push_element_scalar::<SOffsetT>(0x99999999 as SOffsetT);
        let object_vtable_revloc: UOffsetT =
            self.push_element_scalar::<UOffsetT>(0xF0F0F0F0 as UOffsetT);
        //println!("just wrote filler: {:?}", self.get_active_buf_slice());

        // Layout of the data this function will create when a new vtable is
        // needed.
        // --------------------------------------------------------------------
        // vtable starts here
        // | x, x -- vtable len (bytes) [u16]
        // | x, x -- object inline len (bytes) [u16]
        // | x, x -- zero, or num bytes from start of object to field #0   [u16]
        // | ...
        // | x, x -- zero, or num bytes from start of object to field #n-1 [u16]
        // vtable ends here
        // table starts here
        // | x, x, x, x -- offset (negative direction) to our vtable [i32]
        // |               aka "vtableoffset"
        // | -- table inline data begins here, we don't touch it --
        // table ends here -- aka "table_start"
        // --------------------------------------------------------------------
        //
        // Layout of the data this function will create when we re-use an
        // existing vtable.
        //
        // We always serialize this particular vtable, then compare it to the
        // other vtables we know about to see if there is a duplicate. If there
        // is, then we erase the serialized vtable we just made.
        // We serialize it first so that we are able to do byte-by-byte
        // comparisons with already-serialized vtables. This 1) saves
        // bookkeeping space (we only keep revlocs to existing vtables), 2)
        // allows us to convert to little-endian once, then do
        // fast memcmp comparisons, and 3) by ensuring we are comparing real
        // serialized vtables, we can be more assured that we are doing the
        // comparisons correctly.
        //
        // --------------------------------------------------------------------
        // table starts here
        // | x, x, x, x -- offset (negative direction) to an existing vtable [i32]
        // |               aka "vtableoffset"
        // | -- table inline data begins here, we don't touch it --
        // table starts here: aka "table_start"
        // --------------------------------------------------------------------

        // Write a vtable, which consists entirely of voffset_t elements.
        // It starts with the number of offsets, followed by a type id, followed
        // by the offsets themselves. In reverse:
        // Include space for the last offset and ensure empty tables have a
        // minimum size.
        let vtable_len = std::cmp::max(
            self.max_voffset + SIZE_VOFFSET as VOffsetT,
            field_index_to_field_offset(0),
        ) as usize;
        self.fill_big(vtable_len);
        let table_object_size = object_vtable_revloc - table_tail_revloc;
        debug_assert!(table_object_size < 0x10000); // Vtable use 16bit offsets.

        let vt_start_pos = self.cur_idx;
        let vt_end_pos = self.cur_idx + vtable_len;
        {
            let vtfw = &mut VTableWriter::init(&mut self.owned_buf[vt_start_pos..vt_end_pos]);
            vtfw.write_vtable_byte_length(vtable_len as VOffsetT);
            vtfw.write_object_inline_size(table_object_size as VOffsetT);
            for &fl in self.field_locs.iter() {
                let pos: VOffsetT = (object_vtable_revloc - fl.off) as VOffsetT;
                debug_assert_eq!(
                    vtfw.get_field_offset(fl.id),
                    0,
                    "tried to write a vtable field multiple times"
                );
                vtfw.write_field_offset(fl.id, pos);
            }
        }
        let vt_use = {
            let mut ret: usize = self.get_size();

            // LIFO order
            for &vt_rev_pos in self.written_vtable_revpos.iter().rev() {
                let eq = {
                    let this_vt = VTable {
                        buf: &self.owned_buf[..],
                        loc: self.cur_idx,
                    };
                    let other_vt = VTable {
                        buf: &self.owned_buf[..],
                        loc: self.cur_idx + self.get_size() - vt_rev_pos as usize,
                    };
                    other_vt == this_vt
                };
                if eq {
                    VTableWriter::init(&mut self.owned_buf[vt_start_pos..vt_end_pos]).clear();
                    self.cur_idx += vtable_len;
                    ret = vt_rev_pos as usize;
                    break;
                }
            }
            ret
        };
        if vt_use == self.get_size() {
            let n = self.rev_cur_idx();
            self.written_vtable_revpos.push(n);
        }

        {
            let n = self.cur_idx + self.get_size() - object_vtable_revloc as usize;
            let saw = read_scalar::<UOffsetT>(&self.owned_buf[n..n + SIZE_SOFFSET]);
            debug_assert_eq!(saw, 0xF0F0F0F0);
            emplace_scalar::<SOffsetT>(
                &mut self.owned_buf[n..n + SIZE_SOFFSET],
                vt_use as SOffsetT - object_vtable_revloc as SOffsetT,
            );
        }

        self.field_locs.clear();
        self.max_voffset = 0;

        object_vtable_revloc
    }
    pub fn finish_size_prefixed<T>(&mut self, root: Offset<T>, file_identifier: Option<&str>) {
        self.finish_with_opts(root, file_identifier, true);
    }
    pub fn finish<T>(&mut self, root: Offset<T>, file_identifier: Option<&str>) {
        self.finish_with_opts(root, file_identifier, false);
    }
    pub fn finish_minimal<T>(&mut self, root: Offset<T>) {
        self.finish_with_opts(root, None, false);
    }
    // with or without a size prefix changes how we load the data, so finish*
    // functions are split along those lines.
    fn finish_with_opts<T>(
        &mut self,
        root: Offset<T>,
        file_identifier: Option<&str>,
        size_prefixed: bool,
    ) {
        self.assert_not_finished();
        self.assert_not_nested();
        self.written_vtable_revpos.clear();

        let to_align = {
            // for the root offset:
            let a = SIZE_UOFFSET;
            // for the size prefix:
            let b = if size_prefixed { SIZE_UOFFSET } else { 0 };
            // for the file identifier (a string that is not zero-terminated):
            let c = if file_identifier.is_some() {
                FILE_IDENTIFIER_LENGTH
            } else {
                0
            };
            a + b + c
        };

        {
            let ma = self.min_align;
            self.pre_align(to_align, ma);
        }

        if let Some(ident) = file_identifier {
            assert_eq!(ident.len(), FILE_IDENTIFIER_LENGTH);
            self.push_bytes(ident.as_bytes());
        }

        {
            let fwd = self.refer_to(root.value());
            self.push_element_scalar(fwd);
        }

        if size_prefixed {
            let sz = self.get_size() as UOffsetT;
            self.push_element_scalar::<UOffsetT>(sz);
        }
        self.finished = true;
    }

    fn align(&mut self, elem_size: usize) {
        self.track_min_align(elem_size);
        let s = self.get_size();
        self.fill(padding_bytes(s, elem_size));
    }
    pub fn push_element_scalar<T: EndianScalar>(&mut self, t: T) -> UOffsetT {
        self.align(std::mem::size_of::<T>());
        self.push_small(t);
        self.get_size() as UOffsetT
    }
    pub fn place_element_scalar<T: EndianScalar>(&mut self, t: T) -> UOffsetT {
        self.cur_idx -= std::mem::size_of::<T>();
        let cur_idx = self.cur_idx;
        emplace_scalar(&mut self.owned_buf[cur_idx..], t);
        self.get_size() as UOffsetT
    }
    fn push_small<T: EndianScalar>(&mut self, x: T) {
        self.make_space(std::mem::size_of::<T>());
        emplace_scalar(&mut self.owned_buf[self.cur_idx..], x);
    }
    pub fn push_bytes(&mut self, x: &[u8]) -> UOffsetT {
        let n = self.make_space(x.len());
        &mut self.owned_buf[n..n + x.len()].copy_from_slice(x);

        n as UOffsetT
    }
    pub fn push_slot_scalar_indirect_uoffset(
        &mut self,
        slotoff: VOffsetT,
        x: UOffsetT,
        default: UOffsetT,
    ) {
        if x != default {
            let off = self.push_element_scalar_indirect_uoffset(x);
            self.track_field(slotoff, off);
        }
    }
    pub fn push_element_scalar_indirect_uoffset(&mut self, x: UOffsetT) -> UOffsetT {
        let x = self.refer_to(x);
        return self.push_element_scalar(x);
    }
    pub fn push_slot_struct<T: Sized>(&mut self, slotoff: VOffsetT, x: &T) {
        // using to_bytes as a trait makes it easier to mix references into T
        self.assert_nested("");
        let bytes = to_bytes(x);
        self.align(bytes.len());
        self.push_bytes(bytes);
        let sz = self.get_size() as UOffsetT;
        self.track_field(slotoff, sz);
    }
    // Offsets initially are relative to the end of the buffer (downwards).
    // This function converts them to be relative to the current location
    // in the buffer (when stored here), pointing upwards.
    pub fn refer_to(&mut self, off: TailUOffsetT) -> HeadUOffsetT {
        // Align to ensure GetSize() below is correct.
        self.align(SIZE_UOFFSET);
        // Offset must refer to something already in buffer.
        debug_assert!(off > 0);
        debug_assert!(off <= self.get_size() as UOffsetT);
        self.get_size() as UOffsetT - off + SIZE_UOFFSET as UOffsetT
    }
    pub fn push_slot_offset_relative<T>(&mut self, slotoff: VOffsetT, x: Offset<T>) {
        if x.value() == 0 {
            return;
        }
        let rel_off = self.refer_to(x.value());
        self.push_slot_scalar::<UOffsetT>(slotoff, rel_off, 0);
        //AddElement(field, ReferTo(off.o), static_cast<uoffset_t>(0));
        //self.push_uoffset_relative(x.value());
        //self.track_field(slotoff, off);
        //self.push_slot_scalar::<u32>(slotoff, x.value(), 0)
    }
    pub fn push_slot_scalar<T: EndianScalar + std::fmt::Display>(
        &mut self,
        slotoff: VOffsetT,
        x: T,
        default: T,
    ) {
        if x != default {
            let off = self.push_element_scalar(x);
            self.track_field(slotoff, off);
        }
    }

    pub fn make_space(&mut self, want: usize) -> usize {
        self.ensure_space(want);
        self.cur_idx -= want;
        self.cur_idx
    }
    pub fn ensure_space(&mut self, want: usize) -> usize {
        assert!(
            want <= FLATBUFFERS_MAX_BUFFER_SIZE,
            "cannot grow buffer beyond 2 gigabytes"
        );
        while self.unused_ready_space() < want {
            self.grow_owned_buf();
        }
        want
    }
    fn unused_ready_space(&self) -> usize {
        debug_assert!(self.owned_buf.len() >= self.get_size());
        self.owned_buf.len() - self.get_size()
    }
    pub fn finished_bytes(&self) -> &[u8] {
        self.assert_finished();
        &self.owned_buf[self.cur_idx..]
    }
    pub fn required(
        &self,
        tab_revloc: Offset<TableOffset>,
        slot_byte_loc: VOffsetT,
        assert_msg_name: &'static str,
    ) {
        let tab = Table::new(
            &self.owned_buf[..],
            self.cur_idx + (self.get_size() - tab_revloc.0 as usize),
        );
        let o = tab.vtable().get(slot_byte_loc) as usize;
        assert!(o != 0, "missing required field {}", assert_msg_name);
    }
}

#[derive(Debug, PartialEq)]
pub struct Offset<T>(UOffsetT, PhantomData<T>);

// TODO(rw): why do we need to reimplement (with a default impl) Copy to
//           avoid ownership errors?
impl<T> Copy for Offset<T> {}
impl<T> Clone for Offset<T> {
    fn clone(&self) -> Offset<T> {
        Offset::new(self.0.clone())
    }
}

impl<T> std::ops::Deref for Offset<T> {
    type Target = UOffsetT;
    fn deref(&self) -> &UOffsetT {
        &self.0
    }
}
impl<'a, T: 'a> Offset<T> {
    pub fn new(o: UOffsetT) -> Offset<T> {
        Offset {
            0: o,
            1: PhantomData,
        }
    }
    pub fn as_union_value(&self) -> Offset<UnionMarker> {
        Offset::new(self.0)
    }
    pub fn value(&self) -> UOffsetT {
        self.0
    }
}

#[derive(Debug)]
pub struct FollowStart<T>(PhantomData<T>);
impl<'a, T: Follow<'a> + 'a> FollowStart<T> {
    pub fn new() -> Self {
        Self { 0: PhantomData }
    }
    pub fn self_follow(&'a self, buf: &'a [u8], loc: usize) -> T::Inner {
        T::follow(buf, loc)
    }
}
impl<'a, T: Follow<'a>> Follow<'a> for FollowStart<T> {
    type Inner = T::Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        T::follow(buf, loc)
    }
}

#[derive(Debug)]
pub struct ForwardsUOffset<T>(UOffsetT, PhantomData<T>); // data unused

#[derive(Debug)]
pub struct ForwardsVOffset<T>(VOffsetT, PhantomData<T>); // data unused

#[derive(Debug)]
pub struct BackwardsSOffset<T>(SOffsetT, PhantomData<T>); // data unused

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Table<'a> {
    pub buf: &'a [u8],
    pub loc: usize,
}

impl<'a> Follow<'a> for Table<'a> {
    type Inner = Table<'a>;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Table { buf: buf, loc: loc }
    }
}

impl<'a> Table<'a> {
    pub fn new(buf: &'a [u8], loc: usize) -> Self {
        Table { buf: buf, loc: loc }
    }
    #[inline]
    pub fn vtable(&'a self) -> VTable<'a> {
        <BackwardsSOffset<VTable<'a>>>::follow(self.buf, self.loc)
    }
    pub fn get<T: Follow<'a> + 'a>(
        &'a self,
        slot_byte_loc: VOffsetT,
        default: Option<T::Inner>,
    ) -> Option<T::Inner> {
        let o = self.vtable().get(slot_byte_loc) as usize;
        if o == 0 {
            return default;
        }
        Some(<T>::follow(self.buf, self.loc + o))
    }
}

#[derive(Debug)]
pub struct VTable<'a> {
    buf: &'a [u8],
    loc: usize,
}

impl<'a> Follow<'a> for VTable<'a> {
    type Inner = VTable<'a>;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        VTable { buf: buf, loc: loc }
    }
}

impl<'a> PartialEq for VTable<'a> {
    fn eq(&self, other: &VTable) -> bool {
        self.as_bytes().eq(other.as_bytes())
    }
}

impl<'a> VTable<'a> {
    pub fn num_fields(&self) -> usize {
        (self.num_bytes() / SIZE_VOFFSET) - 2
    }
    pub fn num_bytes(&self) -> usize {
        read_scalar_at::<VOffsetT>(self.buf, self.loc) as usize
    }
    pub fn object_inline_num_bytes(&self) -> usize {
        let n = read_scalar_at::<VOffsetT>(self.buf, self.loc + SIZE_VOFFSET);
        n as usize
    }
    pub fn get_field(&self, idx: usize) -> VOffsetT {
        // TODO(rw): distinguish between None and 0?
        if idx > self.num_fields() {
            return 0;
        }
        read_scalar_at::<VOffsetT>(
            self.buf,
            self.loc + SIZE_VOFFSET + SIZE_VOFFSET + SIZE_VOFFSET * idx,
        )
    }
    pub fn get(&self, byte_loc: VOffsetT) -> VOffsetT {
        // TODO(rw): distinguish between None and 0?
        if byte_loc as usize >= self.num_bytes() {
            return 0;
        }
        read_scalar_at::<VOffsetT>(self.buf, self.loc + byte_loc as usize)
    }
    pub fn as_bytes(&self) -> &[u8] {
        let len = self.num_bytes();
        &self.buf[self.loc..self.loc + len]
    }
}

/// VTableWriter compartmentalizes actions needed to create a vtable.
#[derive(Debug)]
pub struct VTableWriter<'a> {
    buf: &'a mut [u8],
}

impl<'a> VTableWriter<'a> {
    pub fn init(buf: &'a mut [u8]) -> Self {
        VTableWriter { buf: buf }
    }

    /// Writes the vtable length (in bytes) into the vtable.
    ///
    /// Note that callers already need to have computed this to initialize
    /// a VTableWriter.
    ///
    /// In debug mode, asserts that the length of the underlying data is equal
    /// to the provided value.
    #[inline]
    pub fn write_vtable_byte_length(&mut self, n: VOffsetT) {
        emplace_scalar::<VOffsetT>(&mut self.buf[..SIZE_VOFFSET], n);
        debug_assert_eq!(n as usize, self.buf.len());
    }

    /// Writes an object length (in bytes) into the vtable.
    #[inline]
    pub fn write_object_inline_size(&mut self, n: VOffsetT) {
        emplace_scalar::<VOffsetT>(&mut self.buf[SIZE_VOFFSET..2 * SIZE_VOFFSET], n);
    }

    /// Gets an object field offset from the vtable. Only used for debugging.
    ///
    /// Note that this expects field offsets (which are like pointers), not
    /// field ids (which are like array indices).
    #[inline]
    pub fn get_field_offset(&self, vtable_offset: VOffsetT) -> VOffsetT {
        let idx = vtable_offset as usize;
        read_scalar::<VOffsetT>(&self.buf[idx..idx + SIZE_VOFFSET])
    }

    /// Writes an object field offset into the vtable.
    ///
    /// Note that this expects field offsets (which are like pointers), not
    /// field ids (which are like array indices).
    #[inline]
    pub fn write_field_offset(&mut self, vtable_offset: VOffsetT, object_data_offset: VOffsetT) {
        let idx = vtable_offset as usize;
        emplace_scalar::<VOffsetT>(&mut self.buf[idx..idx + SIZE_VOFFSET], object_data_offset);
    }

    /// Clears all data in this VTableWriter. Used to cleanly undo a
    /// vtable write.
    #[inline]
    pub fn clear(&mut self) {
        // This is the closest thing to memset in Rust right now.
        let len = self.buf.len();
        let p = self.buf.as_mut_ptr() as *mut u8;
        unsafe {
            std::ptr::write_bytes(p, 0, len);
        }
    }
}

/// Follow is a trait that allows us to access FlatBuffers in a declarative,
/// type safe, and fast way. They compile down to almost no code (after
/// optimizations). Conceptually, Follow lifts the offset-based access
/// patterns of FlatBuffers data into the type system. This trait is used
/// pervasively at read time, to access tables, vtables, vectors, strings, and
/// all other data. At this time, Follow is not utilized much on the write
/// path.
///
/// Writing a new Follow implementation primarily involves deciding whether
/// you want to return data (of the type Self::Inner) or do you want to
/// continue traversing the FlatBuffer.
pub trait Follow<'a> {
    type Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner;
}

impl<'a, T: Follow<'a>> Follow<'a> for ForwardsUOffset<T> {
    type Inner = T::Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        let slice = &buf[loc..loc + SIZE_UOFFSET];
        let off = read_scalar::<u32>(slice) as usize;
        T::follow(buf, loc + off)
    }
}

impl<'a, T: Follow<'a>> Follow<'a> for ForwardsVOffset<T> {
    type Inner = T::Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        let slice = &buf[loc..loc + SIZE_VOFFSET];
        let off = read_scalar::<u16>(slice) as usize;
        T::follow(buf, loc + off)
    }
}

impl<'a, T: Follow<'a>> Follow<'a> for BackwardsSOffset<T> {
    type Inner = T::Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        let slice = &buf[loc..loc + SIZE_SOFFSET];
        let off = read_scalar::<SOffsetT>(slice);
        T::follow(buf, (loc as SOffsetT - off) as usize)
    }
}
impl<'a> Follow<'a> for &'a str {
    type Inner = &'a str;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        let len = read_scalar::<u32>(&buf[loc..loc + SIZE_UOFFSET]) as usize;
        let slice = &buf[loc + SIZE_UOFFSET..loc + SIZE_UOFFSET + len];
        let s = unsafe { std::str::from_utf8_unchecked(slice) };
        s
    }
}

///// Implement direct slice access for byte slices.
//impl<'a> Follow<'a> for &'a [u8] {
//    type Inner = &'a [u8];
//    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
//        let len = read_scalar::<UOffsetT>(&buf[loc..loc + SIZE_UOFFSET]) as usize;
//        let data_buf = &buf[loc + SIZE_UOFFSET .. loc + SIZE_UOFFSET + len];
//        let ptr = data_buf.as_ptr() as *const u8;
//        let s: &'a [u8] = unsafe { std::slice::from_raw_parts(ptr, len) };
//        s
//    }
//}
//
///// Implement direct slice access for GeneratedStruct, which is endian-safe
///// because the structs have accessor functions.
//#[cfg(target_endian="little")]
//impl<'a, T: EndianScalar> Follow<'a> for &'a [T] {
//    type Inner = &'a [T];
//    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
//        let sz = std::mem::size_of::<T>();
//        assert!(sz > 0);
//        let len = read_scalar::<UOffsetT>(&buf[loc..loc + SIZE_UOFFSET]) as usize;
//        let data_buf = &buf[loc + SIZE_UOFFSET .. loc + SIZE_UOFFSET + len * sz];
//        let ptr = data_buf.as_ptr() as *const T;
//        let s: &'a [T] = unsafe { std::slice::from_raw_parts(ptr, len) };
//        s
//    }
//}

pub struct SliceOfGeneratedStruct<T: GeneratedStruct>(T);

/// Implement direct slice access to structs (they are safe on both endiannesses).
impl<'a, T: GeneratedStruct + 'a> Follow<'a> for SliceOfGeneratedStruct<T> {
    type Inner = &'a [T];
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        let sz = std::mem::size_of::<T>();
        assert!(sz > 0);
        let len = read_scalar::<UOffsetT>(&buf[loc..loc + SIZE_UOFFSET]) as usize;
        let data_buf = &buf[loc + SIZE_UOFFSET..loc + SIZE_UOFFSET + len * sz];
        let ptr = data_buf.as_ptr() as *const T;
        let s: &'a [T] = unsafe { std::slice::from_raw_parts(ptr, len) };
        s
    }
}

fn follow_slice_helper<T>(buf: &[u8], loc: usize) -> &[T] {
    let sz = std::mem::size_of::<T>();
    debug_assert!(sz > 0);
    let len = read_scalar::<UOffsetT>(&buf[loc..loc + SIZE_UOFFSET]) as usize;
    let data_buf = &buf[loc + SIZE_UOFFSET..loc + SIZE_UOFFSET + len * sz];
    let ptr = data_buf.as_ptr() as *const T;
    let s: &[T] = unsafe { std::slice::from_raw_parts(ptr, len) };
    s
}

/// Implement direct slice access iff the host is little-endian.
#[cfg(target_endian = "little")]
impl<'a, T: EndianScalar> Follow<'a> for &'a [T] {
    type Inner = &'a [T];
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        follow_slice_helper::<T>(buf, loc)
    }
}

/// Implement Follow for all possible Vectors that have Follow-able elements.
impl<'a, T: Follow<'a> + 'a> Follow<'a> for Vector<'a, T> {
    type Inner = Vector<'a, T>;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Vector {
            0: buf,
            1: loc,
            2: PhantomData,
        }
    }
}

impl<'a, T: 'a> Vector<'a, T> {
    pub fn new(buf: &'a [u8], loc: usize) -> Self {
        Vector {
            0: buf,
            1: loc,
            2: PhantomData,
        }
    }
    pub fn len(&self) -> usize {
        read_scalar::<u32>(&self.0[self.1 as usize..]) as usize
    }
}
impl<'a, T: Follow<'a>> Vector<'a, T> {
    pub fn get(&self, idx: usize) -> T::Inner {
        debug_assert!(idx < read_scalar::<u32>(&self.0[self.1 as usize..]) as usize);
        //println!("entering get({}) with {:?}", idx, &self.0[self.1 as usize..]);
        let sz = std::mem::size_of::<T>();
        debug_assert!(sz > 0);
        T::follow(self.0, self.1 as usize + SIZE_UOFFSET + sz * idx)
    }
}

impl<'a> Vector<'a, u8> {
    pub fn as_bytes(&'a self) -> &'a [u8] {
        <&[u8]>::follow(self.0, self.1)
    }
}

// TODO(rw): endian safety
impl<'a, T: GeneratedStruct> Follow<'a> for &'a T {
    type Inner = &'a T;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        let sz = std::mem::size_of::<T>();
        let buf = &buf[loc..loc + sz];
        //println!("entering follow for Sized ref with {:?}", buf);
        let ptr = buf.as_ptr() as *const T;
        unsafe { &*ptr }
    }
}

pub struct SkipSizePrefix<T>(PhantomData<T>);
impl<'a, T: Follow<'a> + 'a> Follow<'a> for SkipSizePrefix<T> {
    type Inner = T::Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        T::follow(buf, loc + SIZE_SIZEPREFIX)
    }
}

pub struct SkipRootOffset<T>(PhantomData<T>);
impl<'a, T: Follow<'a> + 'a> Follow<'a> for SkipRootOffset<T> {
    type Inner = T::Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        T::follow(buf, loc + SIZE_UOFFSET)
    }
}

pub struct FileIdentifier;
impl<'a> Follow<'a> for FileIdentifier {
    type Inner = &'a [u8];
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        &buf[loc..loc + FILE_IDENTIFIER_LENGTH]
    }
}

pub struct SkipFileIdentifier<T>(PhantomData<T>);
impl<'a, T: Follow<'a> + 'a> Follow<'a> for SkipFileIdentifier<T> {
    type Inner = T::Inner;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        T::follow(buf, loc + FILE_IDENTIFIER_LENGTH)
    }
}

// Implementing Follow using trait bounds (EndianScalar) causes them to
// conflict with the Sized impl. So, they are implemented here on concrete types.
impl<'a> Follow<'a> for bool {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for u8 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for u16 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for u32 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for u64 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for i8 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for i16 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for i32 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for i64 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for f32 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}
impl<'a> Follow<'a> for f64 {
    type Inner = Self;
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        read_scalar_at::<Self>(buf, loc)
    }
}

#[derive(Debug)]
pub struct Vector<'a, T: Sized + 'a>(&'a [u8], usize, PhantomData<T>);

pub fn lifted_follow<'a, T: Follow<'a>>(buf: &'a [u8], loc: usize) -> T::Inner {
    T::follow(buf, loc)
}
pub fn get_root<'a, T: Follow<'a> + 'a>(data: &'a [u8]) -> T::Inner {
    <ForwardsUOffset<T>>::follow(data, 0)
}
pub fn get_size_prefixed_root<'a, T: Follow<'a> + 'a>(data: &'a [u8]) -> T::Inner {
    <SkipSizePrefix<ForwardsUOffset<T>>>::follow(data, 0)
}
pub fn buffer_has_identifier(data: &[u8], ident: &str, size_prefixed: bool) -> bool {
    assert_eq!(ident.len(), FILE_IDENTIFIER_LENGTH);

    let got = if size_prefixed {
        <SkipSizePrefix<SkipRootOffset<FileIdentifier>>>::follow(data, 0)
    } else {
        <SkipRootOffset<FileIdentifier>>::follow(data, 0)
    };

    ident.as_bytes() == got
}
