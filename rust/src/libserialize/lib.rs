//! Support code for encoding and decoding types.

/*
Core encoding and decoding interfaces.
*/

#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://doc.rust-lang.org/favicon.ico",
       html_root_url = "https://doc.rust-lang.org/nightly/",
       html_playground_url = "https://play.rust-lang.org/",
       test(attr(allow(unused_variables), deny(warnings))))]

#![feature(box_syntax)]
#![feature(core_intrinsics)]
#![feature(specialization)]
#![feature(never_type)]
#![feature(nll)]
#![cfg_attr(test, feature(test))]

pub use self::serialize::{Decoder, Encoder, Decodable, Encodable};

pub use self::serialize::{SpecializationError, SpecializedEncoder, SpecializedDecoder};
pub use self::serialize::{UseSpecializedEncodable, UseSpecializedDecodable};

extern crate smallvec;

mod serialize;
mod collection_impls;

pub mod hex;
pub mod json;

pub mod opaque;
pub mod leb128;

mod rustc_serialize {
    pub use serialize::*;
}
