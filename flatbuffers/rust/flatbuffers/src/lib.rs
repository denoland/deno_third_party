//extern crate num_traits;
//use std::convert::AsMut;

//pub trait ToLittleEndian {
//    fn to_le(self) -> Self {
//        self.to_le()
//    }
//}

//impl ToLittleEndian for i16 {}


const FLATBUFFERS_MAX_BUFFER_SIZE: usize = ((1u64 << 32) - 1) as usize;

use std::marker::PhantomData;
//use std::iter::FromIterator;

// enum causes compile error on type mismatch, whereas newtype () would not.
pub enum VectorOffset {}
pub enum StringOffset {}
pub enum ByteStringOffset {}
pub enum UnionOffset {}
pub enum TableOffset {}
pub trait GeneratedStruct {}
pub trait ElementScalar : Sized + PartialEq + Clone {
    fn to_le(self) -> Self;
    fn from_le(self) -> Self;
    //fn eq(&self, rhs: &Self) -> bool;
}
//impl ElementScalar for bool { fn to_le(self) -> bool { u8::to_le(self as u8) as bool } }
impl ElementScalar for bool {
    fn to_le(self) -> bool { self }
    fn from_le(self) -> bool { self }
}
impl ElementScalar for u8 {
    fn to_le(self) -> u8 { u8::to_le(self) }
    fn from_le(self) -> u8 { u8::from_le(self) }
}
impl ElementScalar for i8 {
    fn to_le(self) -> i8 { i8::to_le(self) }
    fn from_le(self) -> i8 { i8::from_le(self) }
}
impl ElementScalar for u16 {
    fn to_le(self) -> u16 { u16::to_le(self) }
    fn from_le(self) -> u16 { u16::from_le(self) }
}
impl ElementScalar for i16 {
    fn to_le(self) -> i16 { i16::to_le(self) }
    fn from_le(self) -> i16 { i16::from_le(self) }
}
impl ElementScalar for u32 {
    fn to_le(self) -> u32 { u32::to_le(self) }
    fn from_le(self) -> u32 { u32::from_le(self) }
}
impl ElementScalar for i32 {
    fn to_le(self) -> i32 { i32::to_le(self) }
    fn from_le(self) -> i32 { i32::from_le(self) }
}
impl ElementScalar for u64 {
    fn to_le(self) -> u64 { u64::to_le(self) }
    fn from_le(self) -> u64 { u64::from_le(self) }
}
impl ElementScalar for i64 {
    fn to_le(self) -> i64 { i64::to_le(self) }
    fn from_le(self) -> i64 { i64::from_le(self) }
}
impl ElementScalar for f32 {
    fn to_le(self) -> f32 { f32::to_le(self) }
    fn from_le(self) -> f32 { self } //f32::from_le(self) }
//  fn eq(&self, rhs: &f32) -> bool {
//      let a: u32 = unsafe { std::mem::transmute(*self) };
//      let b: u32 = unsafe { std::mem::transmute(*rhs) };
//      a == b
//  }
}
impl ElementScalar for f64 {
    fn to_le(self) -> f64 { f64::to_le(self) }
    //fn from_le(self) -> f64 { f64::from_le(self) }
    fn from_le(self) -> f64 { self } //f32::from_le(self) }
//  fn eq(&self, rhs: &f64) -> bool {
//      let a: u64 = unsafe { std::mem::transmute(*self) };
//      let b: u64 = unsafe { std::mem::transmute(*rhs) };
//      a == b
//  }
}

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

#[inline]
pub fn padding_bytes(buf_size: usize, scalar_size: usize) -> usize {
  ((!buf_size) + 1) & (scalar_size - 1)
}
pub fn field_index_to_field_offset(field_id: VOffsetT) -> VOffsetT {
    // Should correspond to what end_table() below builds up.
    let fixed_fields = 2;  // Vtable size and Object Size.
    ((field_id + fixed_fields) * (SIZE_VOFFSET as VOffsetT)) as VOffsetT
}
pub fn field_offset_to_field_index(field_o: VOffsetT) -> VOffsetT {
    assert!(field_o >= 2);
    //if field_o == 0 {
    //    return 0;
    //}
    let fixed_fields = 2;  // Vtable size and Object Size.
    (field_o / (SIZE_VOFFSET as VOffsetT)) - fixed_fields
}
pub fn emplace_scalar<T>(s: &mut [u8], x: T) {
    let sz = std::mem::size_of::<T>();
    let data = unsafe {
        std::slice::from_raw_parts((&x as *const T) as *const u8, sz)
    };

    s[..sz].copy_from_slice(data);
}
pub fn read_scalar_at<T: ElementScalar>(x: &[u8], loc: usize) -> T {
    let buf = &x[loc..loc+std::mem::size_of::<T>()];
    read_scalar(buf)
}
pub fn read_scalar<T: ElementScalar>(x: &[u8]) -> T {
    let p = x.as_ptr();
    let x = unsafe {
        let p2 = std::mem::transmute::<*const u8, *const T>(p);
        (*p2).clone()
    };
    x.from_le()
}

pub trait BufferBacked<'a>{
    // TODO: why isn't a default impl working here?
    fn init_from_bytes(bytes: &'a [u8], pos: usize) -> Self;
}

pub trait VectorGettable {
    fn get(&self, i: usize) -> Self;
}

pub struct Vector<'a, T: Sized + 'a> {
    data: &'a [T],
}
impl<'a, T> Vector<'a, T> {
    pub fn new_from_buf(buf: &'a [u8]) -> Self {
        let len = {
            let p = buf.as_ptr() as *const u32;
            let x = unsafe { *p };
            x.from_le() as usize
        };
        let slice = {
            let p = buf[SIZE_UOFFSET..].as_ptr() as *const T;
            unsafe {
                std::slice::from_raw_parts(p, len)
            }
        };
        Self {
            data: slice,
        }
    }
    pub fn get(&self, idx: usize) -> &T {
        &self.data[idx]
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    //pub fn get(&self, idx: usize) -> &T {
    //    let stride = std::mem::size_of::<T>();
    //    let start = SIZE_UOFFSET;
    //    let loc = start + idx * stride;
    //    let p = self.data[loc..loc + stride].as_ptr() as *const T;
    //    unsafe { &*p }

    //}
    //pub fn len(&self) -> u32 {
    //    let p = self.data.as_ptr() as *const u32;
    //    unsafe { *p }
    //}
}

pub struct Table<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}
impl<'a> Table<'a> {
    pub fn new<'b: 'a>(data: &'b [u8], pos: UOffsetT) -> Self {
        Table {
            data: data,
            pos: pos as usize,
        }
    }

    pub fn get_slot_bool(&self, slotnum: VOffsetT, default: bool) -> bool {
        unimplemented!();
        return true;
    }
    pub fn get_slot_union(&self, slotoff: VOffsetT) -> Option<Table> {
        let o = self.compute_vtable_offset(slotoff) as usize;
        if o == 0 {
            return None;
        }
        let off = o + self.pos;
        let off2 = read_scalar_at::<UOffsetT>(self.data, off) as usize;
        let t2 = Table {
            data: self.data,
            pos: off + off2,
        };
        Some(t2)
    }
    pub fn get_slot_string_unsafe(&'a self, slotnum: VOffsetT) -> Option<&'a str> {
        let o = self.compute_vtable_offset(slotnum) as usize;
        if o == 0 {
            return None;
        }
        let off = o + self.pos;
        let off2 = off + read_scalar_at::<UOffsetT>(self.data, off) as usize;
        let start = off2 + SIZE_UOFFSET as usize;
        let length = read_scalar_at::<UOffsetT>(self.data, off2) as usize;
        let buf = &self.data[start..start+length];
        let s: &str = unsafe {
            let v = std::slice::from_raw_parts(buf.as_ptr(), length);
            // from str::from_utf8_unchecked which is nightly
            &*(v as *const [u8] as *const str)
        };
        Some(s)
    }
    pub fn get_slot_vector<T>(&'a self, slotnum: VOffsetT) -> Option<&'a [T]> {
        let o = self.compute_vtable_offset(slotnum) as usize;
        if o == 0 {
            return None;
        }
        let off = o + self.pos;
        let off2 = off + read_scalar_at::<UOffsetT>(self.data, off) as usize;
        let start = off2 + SIZE_UOFFSET as usize;

        let length = read_scalar_at::<UOffsetT>(self.data, off2) as usize;
        let length_u8 = length * std::mem::size_of::<T>();

        let buf = &self.data[start..start+length_u8];
        let ptr = buf.as_ptr() as *const T;

        let s: &[T] = unsafe {
            std::slice::from_raw_parts(ptr, length)
        };
        Some(s)
    }
    pub fn get_struct_unsafe<T>(&'a self, slotnum: VOffsetT) -> Option<&'a T> {
        let off = self.compute_vtable_offset(slotnum) as usize;
        if off == 0 {
            return None;
        }

        let loc = self.pos + off;
        let buf = &self.data[loc..];//loc+std::mem::size_of::<T>()];
        let ptr = buf.as_ptr() as *const T;
        let x: &T = unsafe {
            &*ptr
        };
        Some(x)


        //read_scalar_at::<T>(self.data, self.pos + off)
    }
    fn get_vtable(&self) -> &[u8] {
        let rev_off = read_scalar_at::<SOffsetT>(self.data, self.pos) as usize;
        &self.data[self.pos - rev_off..]
    }
    fn get_optional_field_offset(&self, slotoff: VOffsetT) -> VOffsetT {
        // The vtable offset is always at the start.
        let vtable = self.get_vtable();
        // The first element is the size of the vtable (fields + type id + itself).
        let vtsize = read_scalar::<VOffsetT>(vtable);
        // If the field we're accessing is outside the vtable, we're reading older
        // data, so it's the same as if the offset was 0 (not present).
        if slotoff < vtsize {
            read_scalar_at::<VOffsetT>(vtable, slotoff as usize)
        } else {
            0
        }
    }
    pub fn get_slot_scalar<T: ElementScalar>(&self, slotoff: VOffsetT, default: T) -> T {
        let field_offset = self.get_optional_field_offset(slotoff);
        if field_offset == 0 {
            default
        } else {
            read_scalar_at::<T>(self.data, self.pos + field_offset as usize)
        }
        //return field_offset ? ReadScalar<T>(data_ + field_offset) : defaultval;
        //let off = self.compute_vtable_offset(slotnum) as usize;
        //if off == 0 {
        //    println!("get_slot_scalar: slotnum={}, off={}, self.data={:?}", slotnum, off, self.data);
        //    return default;
        //}
        //read_scalar_at::<T>(self.data, self.pos + off)
    }
    //pub fn get_struct<T: Sized>(&'a self, slotnum: VOffsetT) -> &'a T {
    //    let field_offset = GetOptionalFieldOffset(field);
    //    auto p = const_cast<uint8_t *>(data_ + field_offset);
    //    return field_offset ? reinterpret_cast<P>(p) : nullptr;
    //}
    pub fn compute_vtable_offset(&self, vtable_offset: VOffsetT) -> VOffsetT {
        let vtable_start = {
            let a = self.pos as SOffsetT;
            let b = read_scalar_at::<SOffsetT>(self.data, self.pos);
            assert!(a - b >= 0, format!("vtable_offset: {}, a: {}, b: {}, self.pos: {}", vtable_offset, a, b, self.pos));
            (a - b) as usize
        };
        let vtsize = read_scalar_at::<VOffsetT>(self.data, vtable_start);
        if vtable_offset >= vtsize {
            return 0;
        }
        read_scalar_at::<VOffsetT>(self.data, vtable_start + vtable_offset as usize)
    }
}
pub struct Struct<'a> {
    _data: &'a [u8],
}
pub struct Verifier {}
impl Verifier {
    pub fn new() -> Self {
        Verifier{}
    }
    pub fn verify<T>(&mut self, _: T) -> bool {
        false
    }
    pub fn verify_buffer<T>(&mut self, _: &'static str) -> bool {
        false
    }
    pub fn verify_vector_of_strings<T>(&mut self, _: T) -> bool {
        false
    }
    pub fn verify_vector_of_tables<T>(&mut self, _: T) -> bool {
        false
    }
    pub fn verify_table<T>(&mut self, _: T) -> bool {
        false
    }
}
pub struct TypeTable {}
pub struct FlatBufferBuilder<'fbb> {
    pub owned_buf: Vec<u8>,
    pub cur_idx: usize,

    vtable: Vec<UOffsetT>,
    vtables: Vec<UOffsetT>,

    nested: bool,
    finished: bool,

    min_align: usize,
    //table_end: UOffsetT,
    max_voffset: VOffsetT,
    _phantom: PhantomData<&'fbb ()>,
}
//impl<T> AsMut<T> for FlatBufferBuilder {
//    fn as_mut(&mut self) -> &mut FlatBufferBuilder {
//        self
//    }
//}
impl<'fbb> FlatBufferBuilder<'fbb> {
    pub fn new() -> Self {
        Self::new_with_capacity(0)
    }
    pub fn new_with_capacity(size: usize) -> Self {
        FlatBufferBuilder{
            owned_buf: vec![0u8; size],
            vtable: Vec::new(),
            vtables: Vec::new(),

            cur_idx: size,

            nested: false,
            finished: false,

            min_align: 0,
            //table_end: 0,

            max_voffset: 0,
            _phantom: PhantomData,
        }
    }
    pub fn start_table(&mut self, num_fields: VOffsetT) -> LabeledUOffsetT<TableOffset> {
        self.assert_not_nested();
        self.nested = true;

        self.vtable.clear();
        //self.vtable.truncate(num_fields as usize);
        self.vtable.resize(num_fields as usize, 0);

        self.min_align = 1;
        LabeledUOffsetT::new(self.rev_cur_idx())

        //self.table_end = self.rev_cur_idx();

        //self.get_size() as UOffsetT
    }
    pub fn store_slot(&mut self, slotoff: VOffsetT) {
        //let i = slotnum as usize;
        //let i = field_offset_to_field_index(slotnum) as usize;
        let slotnum = field_offset_to_field_index(slotoff) as usize;
        assert!(slotnum < self.vtable.len(), "{} !< {}", self.vtable.len(), slotnum);
        self.vtable[slotnum] = self.rev_cur_idx() as UOffsetT;
    }
    pub fn get_buf_slice(&self) -> &[u8] {
        &self.owned_buf[..]
    }
    pub fn get_active_buf_slice(&self) -> &[u8] {
        &self.owned_buf[self.cur_idx..]
    }
    pub fn get_mut_active_buf_slice(&mut self) -> &mut [u8] {
        unreachable!();
        &mut self.owned_buf[self.cur_idx..]
    }
    pub fn reallocate(&mut self, _: usize) {
        unimplemented!()
    }
    pub fn pad(&mut self, n: usize) {
        self.cur_idx -= n;
        for i in 0..n {
            self.owned_buf[self.cur_idx + i] = 0;
        }
    }

    pub fn grow_owned_buf(&mut self) {
        let old_len = self.owned_buf.len();
        let new_len = std::cmp::max(1, old_len * 2);

        assert!(new_len <= FLATBUFFERS_MAX_BUFFER_SIZE,
		        "cannot grow buffer beyond 2 gigabytes");
        //let diff = new_len - old_len;

        self.owned_buf.resize(new_len, 0);
        //self.cur_idx += diff;
        //let ending_active_size = self.get_active_buf_slice().len();
        //assert_eq!(starting_active_size, ending_active_size);
        if new_len == 1 {
            return;
        }

		// calculate the midpoint, and safely copy the old end data to the new
		// end position:
		let middle = new_len / 2;
		{
			let (left, right) = &mut self.owned_buf[..].split_at_mut(middle);
            //println!("foo {}, {:?}, {:?}", middle, &left[..], &right[..]);
			right.copy_from_slice(left);
		}
        // then, zero out the old end data (just to be safe):
        // should be vectorized by the compiler--rust has no stdlib memset.
        for x in &mut self.owned_buf[..middle] {
            *x = 0;
        }

        //let ending_active_size = self.get_active_buf_slice().len();
        //assert_eq!(starting_active_size, ending_active_size);

        //new_len
	}
    //pub fn as_mut(&mut self) -> &mut Self {
    //    self
    //}
    //
    pub fn assert_nested(&self) {
        assert!(self.nested);
        // vtable could be empty (e.g. for all-default values) or in a vector
    }
    pub fn assert_not_nested(&self) {
        assert!(!self.nested);
        assert_eq!(self.vtable.len(), 0);
    }
    pub fn assert_finished(&self) {
        assert!(self.finished);
    }
    pub fn start_vector(&mut self, elemsize: usize, num_elems: usize, alignment: usize) -> UOffsetT {
        self.assert_not_nested();
        self.nested = true;
        self.prep(SIZE_UOFFSET, elemsize*num_elems);
        self.prep(alignment, elemsize*num_elems); // Just in case elemsize is wider than uoffset_t.
        self.rev_cur_idx()
    }
    // Offset relative to the end of the buffer.
    pub fn rev_cur_idx(&self) -> UOffsetT {
        (self.owned_buf.len() - self.cur_idx) as UOffsetT
    }
    pub fn end_vector(&mut self, num_elems: usize) -> LabeledUOffsetT<VectorOffset> {
      self.assert_nested();

      // we already made space for this, so write without PrependUint32
      self.push_element_scalar_no_prep(num_elems as UOffsetT);
      self.nested = false;
      LabeledUOffsetT::new(self.rev_cur_idx())
  }
    pub fn emplace_scalar_in_active_buf<T>(&mut self, at: usize, x: T) {
        let buf = &mut self.get_mut_active_buf_slice();
        emplace_scalar(&mut buf[at..], x)
    }
    pub fn pre_align(&mut self, n: usize, alignment: usize) {
        self.track_min_align(alignment);
        let s = self.get_size() as usize;
        self.fill(padding_bytes(s + n, alignment));
    }
    pub fn prep(&mut self, sz: usize, additional_bytes: usize) {
        // Track the biggest thing we've ever aligned to.
        self.min_align = std::cmp::max(self.min_align, sz);

        // Find the amount of alignment needed such that `size` is properly
        // aligned after `additionalBytes`:
        //println!("prep: sz: {}, addl: {}, owned_buf: {}, cur_idx: {}", sz, additional_bytes, self.owned_buf.len(), self.cur_idx);
        let mut align_size = !(self.owned_buf.len() - self.cur_idx + additional_bytes);
        //println!("prep2: align_size == {}", align_size);
        align_size = {
            let (x, _) = align_size.overflowing_add(1);
            x
        };
        align_size &= (sz - 1);
        //println!("align_size: {}", align_size);

        // Reallocate the buffer if needed:
        while self.cur_idx <= align_size+sz+additional_bytes {
            let old_buf_size = self.owned_buf.len();
            self.grow_owned_buf();
            self.cur_idx += self.owned_buf.len() - old_buf_size;
        }
        // pad:
        for i in 0..align_size {
            self.cur_idx -= 1;
            self.owned_buf[self.cur_idx] = 0;
        }
        //println!("final prep: {}, {}, {}", self.owned_buf.len(), self.cur_idx, align_size);
    }
    pub fn get_size(&self) -> usize {
        self.owned_buf.len() - self.cur_idx
    }
    pub fn fill(&mut self, zero_pad_bytes: usize) {
        self.make_space(zero_pad_bytes);
        let start = self.cur_idx;
        for i in 0..zero_pad_bytes {
            self.owned_buf[start + i] = 0;
        }
    }
    pub fn track_min_align(&mut self, alignment: usize) {
        self.min_align = std::cmp::max(self.min_align, alignment);
    }
    pub fn add_element<T>(&mut self, _: isize, _: T, _: T) -> T {
        unimplemented!()
    }
    pub fn add_offset<T>(&mut self, _: isize, _: LabeledUOffsetT<T>) -> usize {

        unimplemented!()
    }
    pub fn add_struct<T>(&mut self, _: VOffsetT, _: T) {
        unreachable!();
        // TODO: unimplemented!()
    }
    // utf-8 string creation
    pub fn create_string(&mut self, s: &str) -> LabeledUOffsetT<StringOffset> {
        LabeledUOffsetT::new(self.create_byte_string(s.as_bytes()).value())
    }
    pub fn create_byte_string<'a>(&mut self, data: &[u8]) -> LabeledUOffsetT<ByteStringOffset> {
        self.assert_not_nested();
        self.nested = true;
        self.prep(SIZE_UOFFSET, data.len() + 1);

        self.push_element_scalar_no_prep(0u8);
        self.push_bytes_no_prep(data);
        //self.cur_idx -= SIZE_U8;
        //self.owned_buf[self.cur_idx] = 0;
        //self.cur_idx -= data.len();
        //self.owned_buf[self.cur_idx..self.cur_idx+data.len()].copy_from_slice(data);

        LabeledUOffsetT::new(self.end_vector(data.len()).value())

        ////self.pre_align(data.len() + 1, SIZE_UOFFSET);  // Always 0-terminated.
        //self.push_bytes(data);
        //self.push_element_scalar(data.len() as UOffsetT);
        //LabeledUOffsetT::new(self.get_size() as u32)
    }
    pub fn create_byte_vector(&mut self, data: &[u8]) -> LabeledUOffsetT<ByteStringOffset> {
        self.assert_not_nested();
        self.nested = true;
        let l = data.len();
        self.prep(SIZE_UOFFSET, l);

        self.cur_idx -= l;
        self.owned_buf[self.cur_idx..self.cur_idx+l].copy_from_slice(data);

        LabeledUOffsetT::new(self.end_vector(data.len()).value())
    }
    pub fn create_shared_string<'a>(&mut self, _: &'a str) -> LabeledUOffsetT<StringOffset> {
        LabeledUOffsetT::new(0)
    }
    //pub fn create_vector_of_strings<'a, 'b, T: 'b>(&'a mut self, _: &'b [T]) -> Offset<&'b [T]> {
    //pub fn create_vector_of_strings<'a>(&mut self, _: &'a [&'a str]) -> LabeledUOffsetT<VectorOffset<StringOffset>> {
    pub fn create_vector_of_strings<'a>(&mut self, _: &'a [&'a str]) -> LabeledUOffsetT<VectorOffset> {
        LabeledUOffsetT::new(0)
    }
    //pub fn create_vector<T, V: FromIterator<T>>(&mut self, _: V) -> Offset<Vector<T>> {
    pub fn create_vector<'a, T: 'a>(&'a mut self, _: &'a [T]) -> LabeledUOffsetT<&'fbb [T]> {
        LabeledUOffsetT::new(0)
    }
//  //pub fn create_vector_from_fn<'a: 'fbb, 'b, T: 'b, F: FnMut(usize, &mut Self) -> T>(&'fbb mut self, _len: usize, _f: F) -> Offset<&'b [T]> {
    pub fn create_vector_from_fn<F, T>(&mut self, _len: usize, _f: F) -> LabeledUOffsetT<&'fbb [T]>
        where F: FnMut(usize, &mut Self) -> T {
        LabeledUOffsetT::new(0)
    }
//  pub fn create_vector_of_structs<'a, T: 'a>(&'fbb mut self, _: &'a [T]) -> Offset<&'a [T]> {
//      LabeledUOffsetT::new(0)
//  }
//  // TODO probably should not be returning [&T]
    pub fn create_vector_of_sorted_structs<'a, T>(&mut self, _: &'a mut [T]) -> LabeledUOffsetT<&'fbb [T]> {
        LabeledUOffsetT::new(0)
    }
    pub fn create_vector_of_structs_from_fn<T, F>(&mut self, _len: usize, _f: F) -> LabeledUOffsetT<&'fbb [T]>
        where F: FnMut(usize, &mut T) {
        LabeledUOffsetT::new(0)
    }
    pub fn create_vector_of_sorted_tables<'a, T>(&mut self, _: &'a mut [T]) -> LabeledUOffsetT<&'fbb [T]> {
        LabeledUOffsetT::new(0)
    }
    pub fn dump_buf(&self, label: &str) {
        //println!("dump_buf {}: {}/{}: {:?}", label, self.get_size(), self.owned_buf.len(), self.get_active_buf_slice());
    }
    //pub fn end_table3(&mut self, start: UOffsetT) -> UOffsetT {
    //    self.assert_nested();
    //    self.push_element_scalar::<SOffsetT>(0);
    //    let object_offset = b.get_size();
    //}
    pub fn end_table(&mut self, off: LabeledUOffsetT<TableOffset>) -> LabeledUOffsetT<TableOffset> {
        //println!("1/3");
        self.assert_nested();
        //println!("2/3");
        let n = self.write_vtable(off.value());
        //println!("3/3");
        self.nested = false;
        let o = LabeledUOffsetT::new(n);
        o
    }
    pub fn write_vtable(&mut self, table_end: UOffsetT) -> UOffsetT {
        self.push_soffset_relative(0);

        let table_offset = self.rev_cur_idx();

        // Trim vtable of trailing zeroes.
        for i in (0..self.vtable.len()).rev() {
            if self.vtable[i] != 0 {
                break
            };
            self.vtable.truncate(i);
        }

        let existing_vtable = false;
        if !existing_vtable {
            // Did not find a vtable, so write this one to the buffer.

            // Write out the current vtable in reverse, because
            // serialization occurs in last-first order:
            // (we cannot use an iterator here due to false borrowing.)
            for i in (0..self.vtable.len()).rev() {
                let val = self.vtable[i]; // prevent underflow of unsigned type
                let vt_off = if val == 0 {
                    0
                } else {
                    // Forward reference to field;
                    // use 32bit number to assert no overflow:
                    table_offset - val
                };
                //println!("pushing VOffsetT {} at index {} (val = {}, table_offset = {})", off, i, val, table_offset);
                self.push_element_scalar::<VOffsetT>(vt_off as VOffsetT);
            }

            // The two metadata fields are written last.

            // First, store the object bytesize:
            let table_size = table_offset - table_end;
            self.push_element_scalar::<VOffsetT>(table_size as VOffsetT);

            // Second, store the vtable bytesize:
            let vtable_size = (self.vtable.len() + VTABLE_METADATA_FIELDS) * SIZE_VOFFSET;
            //let vtable_size = field_index_to_field_offset(self.vtable.len() as VOffsetT);
            self.push_element_scalar::<VOffsetT>(vtable_size as VOffsetT);

            // Next, write the offset to the new vtable in the
            // already-allocated SOffsetT at the beginning of this object:
            let table_start = self.owned_buf.len() as SOffsetT - table_offset as SOffsetT;
            //println!("before emplace: {} {:?}", cur_idx, &self.owned_buf[..]);
            {
                //assert_eq!(0, read_scalar_at::<SOffsetT>(&self.owned_buf[..], table_start as usize));
                let n = self.rev_cur_idx();
                emplace_scalar(&mut self.owned_buf[table_start as usize..],
                               n as SOffsetT - table_offset as SOffsetT);
            }
            //println!("after emplace:  {} {:?}", cur_idx, &self.owned_buf[..]);

            // Finally, store this vtable in memory for future
            // deduplication:
            //{
            //    let n = self.rev_cur_idx();
            //    self.vtables.push(n);
            //}

        //println!("final vtable: {:?}", self.vtable);
            self.vtable.truncate(0);

            return table_offset;
        }

        //// empty vtable for now
        //let table_size = table_offset - off;
        //self.push_element_scalar::<VOffsetT>(table_size as VOffsetT);
        //let vtable_size = (0 + VTABLE_METADATA_FIELDS) * SIZE_VOFFSET;
        //self.push_element_scalar::<VOffsetT>(vtable_size as VOffsetT);
        //let table_start = self.owned_buf.len() as SOffsetT - table_offset as SOffsetT;
        //{
        //    let n = self.rev_cur_idx() as SOffsetT - table_offset as SOffsetT;
        //    emplace_scalar::<SOffsetT>(&mut self.owned_buf[table_start as usize..],n);
        //}

        0
    }
    pub fn end_table_old(&mut self, start: UOffsetT) -> UOffsetT {
        unreachable!();
        // self.assert_nested();

        // let vtableoffsetloc = self.push_element_scalar::<SOffsetT>(0);
        // self.dump_buf(&format!("pushed empty vtableoffsetloc {}", vtableoffsetloc));
        // self.max_voffset = std::cmp::max(self.max_voffset + SIZE_VOFFSET as VOffsetT,
        //                                  field_index_to_field_offset(0));
        // let to_fill = self.max_voffset as usize;
        // self.fill(to_fill);
        // self.dump_buf(&format!("filled {}", to_fill));

        // let table_object_size = vtableoffsetloc - start;
        // assert!(table_object_size < 0x10000);  // Vtable use 16bit offsets.

        // {
        //     let n = table_object_size as VOffsetT;
        //     emplace_scalar::<VOffsetT>(&mut self.get_mut_active_buf_slice()[SIZE_VOFFSET..], n);
        //     self.dump_buf(&format!("after placing table_object_size {}", n));
        // }
        // {
        //     let n = self.max_voffset as VOffsetT;
        //     emplace_scalar::<VOffsetT>(&mut self.get_mut_active_buf_slice(), n);
        //     self.dump_buf(&format!("after placing max_voffset {}", n));
        // }

        // let vt_use = self.get_size();
        // //println!("vt_use at start: {}", vt_use);
        // // TODO write vtable

        // {
        //     let n = (vt_use as SOffsetT) - (vtableoffsetloc as SOffsetT);
        //     //let i = self.get_size() - vtableoffsetloc as usize;
        //     let buf = &mut self.get_mut_active_buf_slice();
        //     //let i = buf.len() - vtableoffsetloc as usize;
        //     let i = vtableoffsetloc as usize;
        //     //println!("writing vt_use... {} at {} -- {:?}", n, i, buf);
        //     emplace_scalar::<SOffsetT>(&mut buf[i..], n);
        // }

        // self.nested = false;

        // vtableoffsetloc as UOffsetT
    }

    pub fn required<T>(&self, _: &LabeledUOffsetT<T>, _: VOffsetT) {
        //TODO: unimplemented!()
    }
    pub fn finish<T>(&mut self, root: LabeledUOffsetT<T>) {
        self.assert_not_nested();
        let min_align = self.min_align;
        self.prep(min_align, SIZE_UOFFSET);
        self.push_element_scalar_indirect_uoffset(root.value());
        self.finished = true;
    }
    pub fn finish_with_identifier<'a, T>(&'a mut self, root: LabeledUOffsetT<T>, name: &'static str) {
        self.finish(root)
    }

    pub fn release_buffer_pointer(&mut self) -> DetachedBuffer  {
       //self.Finished();
       // self.buf_.release();
       DetachedBuffer{}
    }

    pub fn push_element_bool(&mut self, b: bool) -> UOffsetT {
        unimplemented!();
        self.push_element_scalar(b as u8)
    }
    fn align(&mut self, elem_size: usize) {
        let delta = self.cur_idx % elem_size;
        self.cur_idx -= delta;
    }
    pub fn push_element_scalar_no_prep<T: ElementScalar>(&mut self, t: T) -> UOffsetT {
        //let t = t.to_le(); // convert to little-endian
        self.cur_idx -= std::mem::size_of::<T>();
        emplace_scalar::<T>(&mut self.owned_buf[self.cur_idx..], t);
        self.cur_idx as UOffsetT
    }
    pub fn push_element_scalar<T: ElementScalar>(&mut self, t: T) -> UOffsetT {
        self.prep(std::mem::size_of::<T>(), 0);
        self.cur_idx -= std::mem::size_of::<T>();
        emplace_scalar::<T>(&mut self.owned_buf[self.cur_idx..], t);
        self.cur_idx as UOffsetT
    }
    pub fn place_element_scalar<T: ElementScalar>(&mut self, t: T) {
        let t = t.to_le(); // convert to little-endian
        self.cur_idx -= std::mem::size_of::<T>();
        let cur_idx = self.cur_idx;
        emplace_scalar(&mut self.owned_buf[cur_idx..], t);

    }
    pub fn push_soffset_relative(&mut self, off: SOffsetT) {
        self.prep(SIZE_SOFFSET, 0);
        //self.pre_align(SIZE_SOFFSET, 0);
        //self.align(SIZE_SOFFSET);
        //self.align(std::mem::size_of::<SOffsetT>());
        assert!(off <= self.rev_cur_idx() as SOffsetT, "logic error in offsets");
        let off2 = (self.rev_cur_idx() as SOffsetT) - (off as SOffsetT) + (SIZE_SOFFSET as SOffsetT);
        //println!("off2: {}", off2);
        //self.dump_buf("emplace off2");
        self.push_element_scalar_no_prep(off2);
        //emplace_scalar(&mut self.owned_buf[start..start+SIZE_SOFFSET], off2);
    }
    fn push_uoffset_relative(&mut self, off: UOffsetT) {
        self.prep(SIZE_UOFFSET, 0);
        assert!(off <= self.rev_cur_idx() as UOffsetT, "logic error in offsets");
        let off2 = (self.rev_cur_idx() as UOffsetT) - (off as UOffsetT) + (SIZE_UOFFSET as UOffsetT);
        //println!("off2: {}", off2);
        //self.dump_buf("emplace off2");
        self.push_element_scalar_no_prep(off2);
        //emplace_scalar(&mut self.owned_buf[start..start+SIZE_SOFFSET], off2);
    }
    pub fn push_bytes_no_prep(&mut self, x: &[u8]) -> UOffsetT {
        let l = x.len();
        self.cur_idx -= l;
        &mut self.owned_buf[self.cur_idx..self.cur_idx+l].copy_from_slice(x);

        self.cur_idx as UOffsetT
    }
    pub fn push_bytes(&mut self, x: &[u8]) -> UOffsetT {
        unreachable!();
        let n = self.make_space(x.len());
        &mut self.owned_buf[n..n+x.len()].copy_from_slice(x);

        n as UOffsetT
    }
    pub fn push_slot_scalar_indirect_uoffset(&mut self, slotoff: VOffsetT, x: UOffsetT, default: UOffsetT) {
        if x != default {
            self.push_element_scalar_indirect_uoffset(x);
            self.store_slot(slotoff);
        }
    }
    pub fn push_element_scalar_indirect_uoffset(&mut self, x: UOffsetT) {
        self.prep(std::mem::size_of::<UOffsetT>(), 0);
        assert!(x <= self.rev_cur_idx() as UOffsetT, "logic error");
        let off2 = self.rev_cur_idx() as UOffsetT - x + SIZE_UOFFSET as UOffsetT;
        self.push_element_scalar_no_prep::<UOffsetT>(off2);
    }
    pub fn push_slot_bool(&mut self, slotnum: VOffsetT, x: bool, default: bool) {
        unimplemented!();
        self.push_slot_scalar(slotnum, x as u8, default as u8);
    }
    pub fn push_slot_struct<T: GeneratedStruct>(&mut self, slotoff: VOffsetT, x: Option<&T>) {
        self.assert_nested();
        if let Some(ref valref) = x {
            let ptr = *valref as *const T as *const u8;
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts::<u8>(ptr, std::mem::size_of::<T>())
            };
            self.prep(bytes.len(), 0);
            self.push_bytes_no_prep(bytes);
            //if x != self.rev_cur_idx() {
            //    panic!("structs must be written inside a table");
            //}
            self.store_slot(slotoff);
        }
    }
    // Offsets initially are relative to the end of the buffer (downwards).
    // This function converts them to be relative to the current location
    // in the buffer (when stored here), pointing upwards.
    pub fn refer_to(&mut self, off: UOffsetT) -> UOffsetT {
        // Align to ensure GetSize() below is correct.
        self.align(SIZE_UOFFSET);
        // Offset must refer to something already in buffer.
        assert!(off > 0);
        assert!(off <= self.get_size() as UOffsetT);
        self.get_size() as UOffsetT - off + SIZE_UOFFSET as UOffsetT
    }
    pub fn push_slot_labeled_uoffset_relative_from_option<T>(&mut self, slotoff: VOffsetT, x: Option<LabeledUOffsetT<T>>) {
        if let Some(o) = x {
            self.push_slot_labeled_uoffset_relative(slotoff, o)
        }
    }
    pub fn push_slot_labeled_uoffset_relative<T>(&mut self, slotoff: VOffsetT, x: LabeledUOffsetT<T>) {
        if x.value() == 0 {
            return;
        }
        let rel_off = self.refer_to(x.value());
        self.push_slot_scalar::<UOffsetT>(slotoff, rel_off, 0);
        //AddElement(field, ReferTo(off.o), static_cast<uoffset_t>(0));
        //self.push_uoffset_relative(x.value());
        self.store_slot(slotoff);
        //self.push_slot_scalar::<u32>(slotoff, x.value(), 0)
    }
    pub fn push_slot_scalar<T: ElementScalar + std::fmt::Display>(&mut self, slotoff: VOffsetT, x: T, default: T) {
        //println!("push_slot_scalar: slotnum={}, x={}, default={}, get_active_buf_slice={:?}", slotnum, x, default, self.get_active_buf_slice());
        if x != default {
           //// println!("pushing slot scalar {} != {}", x, default);
            self.push_element_scalar(x);
            //self.prep(std::mem::size_of::<T>(), 0);
            //emplace_scalar(&mut self.owned_buf[self.cur_idx..], x);
            //self.push_element_scalar(x);
            self.store_slot(slotoff);
        }
    }

    pub fn push<T: Sized>(&mut self, x: T) {
        unreachable!();
        //println!("start of push: {}", self.cur_idx);
        let s = std::mem::size_of::<T>();
        //println!("make space {}", s);
        let n = self.make_space(s);
        {
            let start = self.cur_idx;
            emplace_scalar(&mut self.owned_buf[start..start+s], x);
        }
        //println!("after push: {} {:?} {:?}", self.cur_idx, self.get_active_buf_slice(), &self.owned_buf[..]);
    }

    pub fn release(&mut self) {
        //DetachedBuffer fb(allocator_, own_allocator_, buf_, reserved_, cur_,
        //                  size());
        //allocator_ = nullptr;
        //own_allocator_ = false;
        //buf_ = nullptr;
        //clear();
        //return fb;
    }

    pub fn make_space(&mut self, want: usize) -> usize {
        self.ensure_space(want);
        self.cur_idx -= want;
        self.cur_idx
    }
    pub fn ensure_space(&mut self, want: usize) -> usize {
        assert!(want <= FLATBUFFERS_MAX_BUFFER_SIZE,
		        "cannot grow buffer beyond 2 gigabytes");
        while self.cur_idx < want {
            //println!("growing: {} < {}: {:?}", self.cur_idx, want, self.get_active_buf_slice());
            self.grow_owned_buf();
            //println!("grew to: {}, {}, {:?}", self.cur_idx, self.owned_buf.len(), self.get_active_buf_slice());
        }
        want
    }
    pub fn finished_bytes(&self) -> &[u8] {
        self.assert_finished();
        &self.owned_buf[self.cur_idx..]
    }
}
pub trait UOffsetTTrait {}
pub trait OffsetTTrait {}
pub trait VOffsetTTrait {}
pub type UOffsetT = u32;
pub type SOffsetT = i32;
pub type VOffsetT = i16;

//pub type String<'a> = &'a str;
pub type Void<'a> = &'a [u8];
//pub struct Vector<T>  {
//    phantom: PhantomData<T>,
//}

//pub struct LabeledUOffsetT<T> (usize, PhantomData<T>);
pub struct VectorLabeledUOffsetT<T> (usize, PhantomData<T>);
//pub struct ULabeledUOffsetT<T> (u32, PhantomData<T>);

//impl<T> LabeledUOffsetT<T> {
//    pub fn new(o: usize) -> Self {
//        Offset(o, PhantomData)
//    }
//    pub fn union(&self) -> Offset<Void> {
//        LabeledUOffsetT::new(self.0)
//    }
//}
//impl<T> ULabeledUOffsetT<T> {
//    pub fn new(o: u32) -> Self {
//        UOffset(o, PhantomData)
//    }
//}
pub struct LabeledUOffsetT<T> (UOffsetT, PhantomData<T>);
impl<T> Copy for LabeledUOffsetT<T> { } // TODO: why does deriving Copy cause ownership errors?
impl<T> Clone for LabeledUOffsetT<T> {
    fn clone(&self) -> LabeledUOffsetT<T> {
        LabeledUOffsetT::new(self.0.clone())
    }
}

impl<T> std::ops::Deref for LabeledUOffsetT<T> {
    type Target = UOffsetT;
    fn deref(&self) -> &UOffsetT {
        &self.0
    }
}
impl<T> LabeledUOffsetT<T> {
    pub fn new(o: UOffsetT) -> Self {
        LabeledUOffsetT(o, PhantomData)
    }
    pub fn union(&self) -> LabeledUOffsetT<UnionOffset> {
        LabeledUOffsetT::new(self.0)
    }
    pub fn value(&self) -> UOffsetT {
        self.0
    }
}

//impl<T> From<usize> for ULabeledUOffsetT<T> { fn from(n: usize) -> Self { ULabeledUOffsetT::new(n) } }
//impl<T> From<isize> for LabeledUOffsetT<T> { fn from(n: isize) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<u8> for LabeledUOffsetT<T>  { fn from(n: u8)  -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<u16> for LabeledUOffsetT<T> { fn from(n: u16) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<u32> for LabeledUOffsetT<T> { fn from(n: u32) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<u64> for LabeledUOffsetT<T> { fn from(n: u64) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<i8> for LabeledUOffsetT<T>  { fn from(n: i8)  -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<i16> for LabeledUOffsetT<T> { fn from(n: i16) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<i32> for LabeledUOffsetT<T> { fn from(n: i32) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<i64> for LabeledUOffsetT<T> { fn from(n: i64) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<usize> for LabeledUOffsetT<T> { fn from(n: usize) -> Self { LabeledUOffsetT::new(n) } }
//impl<T> From<isize> for LabeledUOffsetT<T> { fn from(n: isize) -> Self { LabeledUOffsetT::new(n) } }
//impl From<usize> for Offset<u16> { fn from(n: usize) -> Self { LabeledUOffsetT::new(n) } }
//impl From<usize> for Offset<u32> { fn from(n: usize) -> Self { LabeledUOffsetT::new(n) } }
//impl From<usize> for Offset<u64> { fn from(n: usize) -> Self { LabeledUOffsetT::new(n) } }
//impl From<usize> for Offset<f32> { fn from(n: usize) -> Self { LabeledUOffsetT::new(n) } }
//impl From<usize> for Offset<f64> { fn from(n: usize) -> Self { LabeledUOffsetT::new(n) } }

pub fn verify_table_start(_: &Verifier) -> bool {
    false
}
//pub fn endian_scalar<T: num_traits::int::PrimInt>(x: T) -> T {
//    x.to_le()
//}
pub fn endian_scalar<T>(x: T) -> T {
    x
    //x.to_le()
}
pub fn write_scalar<S, T>(_: S, _: T) -> ! {
    unimplemented!()
}
pub fn set_field<T>(_: isize, _: T, _: T) -> bool {
    unimplemented!()
}
pub fn verify_field<T>(_: &Verifier, _: VOffsetT) -> bool {
    false
}
pub fn verify_offset(_: &Verifier, _: VOffsetT) -> ! {
    unimplemented!()
}
pub fn verify_offset_required(_: &Verifier, _: VOffsetT) -> ! {
    unimplemented!()
}
//pub fn get_root<'a, 'b: 'a, T: BufferBacked<'a>>(bytes: &'b [u8]) -> T {
pub fn get_root<'a, T: BufferBacked<'a> + 'a>(bytes: &'a [u8]) -> T {
	let n = read_scalar::<UOffsetT>(bytes) as usize;
    println!("get_root n: {}, len of bytes: {}", n, bytes.len());
    T::init_from_bytes(bytes, n)

    //let ptr = obj_bytes.as_ptr() as *const T;
    //println!("bytes: {}, n: {}, xx: {:?}, ptr: {}", bytes.len(), n, &bytes[..8], ptr as usize);
    ////unimplemented!();
    //unsafe {
    //    &*ptr
    //}
}
pub fn get_mutable_root<T>(_: &[u8]) -> T {
    unimplemented!()
}
pub fn get_struct_mut<T>(_: VOffsetT) -> T {
    unimplemented!()
}
pub fn get_field<T: ElementScalar>(slotnum: VOffsetT, default: T) -> T {
    unreachable!();
    //let off = self.compute_vtable_offset(slotnum);
    //if off == 0 {
    //    return default;
    //}
    //read_scalar_at::<T>(&self.data, off as usize)
}
//pub fn get_field<T: num_traits::Num>(_: isize, _: T) -> T {
//    unimplemented!()
//}
//pub fn get_field_mut<T: num_traits::Num>(_: isize, _: T) -> T {
//    unimplemented!()
//}
pub fn get_pointer<'a, T: 'a>(_: VOffsetT) -> &'a T {
    unimplemented!()
}
pub fn get_pointer_mut<'a, T: 'a>(_: VOffsetT) -> &'a mut T {
    unimplemented!()
}
pub fn buffer_has_identifier<S, T>(_: S, _: T) -> bool {
    false
}
pub struct DetachedBuffer {}
pub mod flexbuffers {
    pub struct Reference {}
pub fn get_root<T>(_: &[u8], _: isize) -> T {
    unimplemented!()
}

}
