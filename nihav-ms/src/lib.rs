extern crate nihav_core;
extern crate nihav_codec_support;

#[allow(clippy::needless_range_loop)]
#[allow(clippy::single_match)]
#[allow(clippy::verbose_bit_mask)]
mod codecs;
pub use crate::codecs::ms_register_all_codecs;
pub use crate::codecs::ms_register_all_encoders;
