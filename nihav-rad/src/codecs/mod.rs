use nihav_core::codecs::*;

macro_rules! validate {
    ($a:expr) => { if !$a { println!("check failed at {}:{}", file!(), line!()); return Err(DecoderError::InvalidData); } };
}

#[cfg(any(feature="decoder_smkaud", feature="decoder_smkvid"))]
mod smacker;
#[cfg(feature="decoder_binkaud")]
mod binkaud;
#[cfg(feature="decoder_binkvid")]
mod binkvid;
#[cfg(feature="decoder_bink2")]
mod bink2;

const RAD_CODECS: &[DecoderInfo] = &[
#[cfg(feature="decoder_smkaud")]
    DecoderInfo { name: "smacker-audio", get_decoder: smacker::get_decoder_audio },
#[cfg(feature="decoder_smkvid")]
    DecoderInfo { name: "smacker-video", get_decoder: smacker::get_decoder_video },
#[cfg(feature="decoder_binkaud")]
    DecoderInfo { name: "bink-audio-dct", get_decoder: binkaud::get_decoder_dct },
#[cfg(feature="decoder_binkaud")]
    DecoderInfo { name: "bink-audio-rdft", get_decoder: binkaud::get_decoder_rdft },
#[cfg(feature="decoder_binkvid")]
    DecoderInfo { name: "bink-video", get_decoder: binkvid::get_decoder },
#[cfg(feature="decoder_bink2")]
    DecoderInfo { name: "bink2-video", get_decoder: bink2::get_decoder },
];

/// Registers all available codecs provided by this crate.
pub fn rad_register_all_codecs(rd: &mut RegisteredDecoders) {
    for decoder in RAD_CODECS.iter() {
        rd.add_decoder(decoder.clone());
    }
}
