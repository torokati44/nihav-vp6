use nihav_core::formats;
use nihav_core::codecs::*;
use nihav_core::io::byteio::*;
use std::str::FromStr;


pub fn get_decoder_video() -> Box<NADecoder> {
    unimplemented!();
}

struct BMV3AudioDecoder {
    ainfo:      NAAudioInfo,
    chmap:      NAChannelMap,
    pred:       [i16; 2],
    nframes:    usize,
}

impl BMV3AudioDecoder {
    fn new() -> Self {
        Self {
            ainfo:      NAAudioInfo::new(0, 1, formats::SND_S16P_FORMAT, 0),
            chmap:      NAChannelMap::new(),
            pred:       [0; 2],
            nframes:    0,
        }
    }
}

fn decode_block(mode: u8, src: &[u8], dst: &mut [i16], mut pred: i16) -> i16 {
    let steps = &BMV_AUDIO_STEPS[mode as usize];
    let mut val2 = 0;
    for i in 0..10 {
        let val = (src[i * 2 + 0] as usize) + (src[i * 2 + 1] as usize) * 256;
        pred = pred.wrapping_add(steps[(val >> 10) & 0x1F]);
        dst[i * 3 + 0] = pred;
        pred = pred.wrapping_add(steps[(val >>  5) & 0x1F]);
        dst[i * 3 + 1] = pred;
        pred = pred.wrapping_add(steps[(val >>  0) & 0x1F]);
        dst[i * 3 + 2] = pred;
        val2 = (val2 << 1) | (val >> 15);
    }
    pred = pred.wrapping_add(steps[(val2 >> 5) & 0x1F]);
    dst[3 * 10 + 0] = pred;
    pred = pred.wrapping_add(steps[(val2 >> 0) & 0x1F]);
    dst[3 * 10 + 1] = pred;
    pred
}

impl NADecoder for BMV3AudioDecoder {
    fn init(&mut self, info: Rc<NACodecInfo>) -> DecoderResult<()> {
        if let NACodecTypeInfo::Audio(ainfo) = info.get_properties() {
            self.ainfo = NAAudioInfo::new(ainfo.get_sample_rate(), ainfo.get_channels(), formats::SND_S16P_FORMAT, 32);
            self.chmap = NAChannelMap::from_str("L,R").unwrap();
            Ok(())
        } else {
            Err(DecoderError::InvalidData)
        }
    }
    fn decode(&mut self, pkt: &NAPacket) -> DecoderResult<NAFrameRef> {
        let info = pkt.get_stream().get_info();
        if let NACodecTypeInfo::Audio(_) = info.get_properties() {
            let pktbuf = pkt.get_buffer();
            validate!(pktbuf.len() > 1);
            let samples = (pktbuf.len() / 41) * 32;
            let abuf = alloc_audio_buffer(self.ainfo, samples, self.chmap.clone())?;
            let mut adata = abuf.get_abuf_i16().unwrap();
            let off1 = adata.get_offset(1);
            let mut dst = adata.get_data_mut();
            let mut first = pktbuf[0] == 0;
            let psrc = &pktbuf[1..];
            for (n, src) in psrc.chunks_exact(41).enumerate() {
                let aoff0 = n * 32;
                let aoff1 = aoff0 + off1;
                if first {
                    let mode = src[40];
                    self.pred[0] = decode_block(mode >> 4, &src[0..], &mut dst[aoff0..], self.pred[0]);
                    self.pred[1] = decode_block(mode & 0xF, &src[20..], &mut dst[aoff1..], self.pred[1]);
                } else {
                    let mode = src[0];
                    self.pred[0] = decode_block(mode >> 4, &src[1..], &mut dst[aoff0..], self.pred[0]);
                    self.pred[1] = decode_block(mode & 0xF, &src[21..], &mut dst[aoff1..], self.pred[1]);
                }
                first = !first;
            }
            self.nframes += 1;
            let mut frm = NAFrame::new_from_pkt(pkt, info, abuf);
            frm.set_duration(Some(samples as u64));
            frm.set_keyframe(false);
            Ok(Rc::new(RefCell::new(frm)))
        } else {
            Err(DecoderError::InvalidData)
        }
    }
}

pub fn get_decoder_audio() -> Box<NADecoder> {
    Box::new(BMV3AudioDecoder::new())
}

#[cfg(test)]
mod test {
    use nihav_core::codecs::RegisteredDecoders;
    use nihav_core::demuxers::RegisteredDemuxers;
    use nihav_core::test::dec_video::*;
    use crate::codecs::game_register_all_codecs;
    use crate::demuxers::game_register_all_demuxers;
    #[test]
    fn test_bmv_video() {
        let mut dmx_reg = RegisteredDemuxers::new();
        game_register_all_demuxers(&mut dmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        game_register_all_codecs(&mut dec_reg);

        let file = "assets/Game/DW3-Loffnote.bmv";
        test_file_decoding("bmv3", file, Some(40), true, false, None, &dmx_reg, &dec_reg);
    }
    #[test]
    fn test_bmv_audio() {
        let mut dmx_reg = RegisteredDemuxers::new();
        game_register_all_demuxers(&mut dmx_reg);
        let mut dec_reg = RegisteredDecoders::new();
        game_register_all_codecs(&mut dec_reg);

        let file = "assets/Game/DW3-Loffnote.bmv";
        test_decode_audio("bmv3", file, None, "bmv3", &dmx_reg, &dec_reg);
    }
}

const BMV_AUDIO_STEPS: [[i16; 32]; 16] = [
    [
         0x0000,  0x0400,  0x0800,  0x0C00,  0x1000,  0x1400,  0x1800,  0x1C00,
         0x2000,  0x2400,  0x2800,  0x2C00,  0x3000,  0x3400,  0x3800,  0x3C00,
        -0x4000, -0x3C00, -0x3800, -0x3400, -0x3000, -0x2C00, -0x2800, -0x2400,
        -0x2000, -0x1C00, -0x1800, -0x1400, -0x1000, -0x0C00, -0x0800, -0x0400
    ], [
         0x0000,  0x0200,  0x0400,  0x0600,  0x0800,  0x0A00,  0x0C00,  0x0E00,
         0x1000,  0x1200,  0x1400,  0x1600,  0x1800,  0x1A00,  0x1C00,  0x1E00,
        -0x2000, -0x1E00, -0x1C00, -0x1A00, -0x1800, -0x1600, -0x1400, -0x1200,
        -0x1000, -0x0E00, -0x0C00, -0x0A00, -0x0800, -0x0600, -0x0400, -0x0200
    ], [
         0x0000,  0x0100,  0x0200,  0x0300,  0x0400,  0x0500,  0x0600,  0x0700,
         0x0800,  0x0900,  0x0A00,  0x0B00,  0x0C00,  0x0D00,  0x0E00,  0x0F00,
        -0x1000, -0x0F00, -0x0E00, -0x0D00, -0x0C00, -0x0B00, -0x0A00, -0x0900,
        -0x0800, -0x0700, -0x0600, -0x0500, -0x0400, -0x0300, -0x0200, -0x0100
    ], [
         0x000,  0x080,  0x100,  0x180,  0x200,  0x280,  0x300,  0x380,
         0x400,  0x480,  0x500,  0x580,  0x600,  0x680,  0x700,  0x780,
        -0x800, -0x780, -0x700, -0x680, -0x600, -0x580, -0x500, -0x480,
        -0x400, -0x380, -0x300, -0x280, -0x200, -0x180, -0x100, -0x080
    ], [
         0x000,  0x048,  0x090,  0x0D8,  0x120,  0x168,  0x1B0,  0x1F8,
         0x240,  0x288,  0x2D0,  0x318,  0x360,  0x3A8,  0x3F0,  0x438,
        -0x480, -0x438, -0x3F0, -0x3A8, -0x360, -0x318, -0x2D0, -0x288,
        -0x240, -0x1F8, -0x1B0, -0x168, -0x120, -0x0D8, -0x090, -0x048
    ], [
         0x000,  0x030,  0x060,  0x090,  0x0C0,  0x0F0,  0x120,  0x150,
         0x180,  0x1B0,  0x1E0,  0x210,  0x240,  0x270,  0x2A0,  0x2D0,
        -0x300, -0x2D0, -0x2A0, -0x270, -0x240, -0x210, -0x1E0, -0x1B0,
        -0x180, -0x150, -0x120, -0x0F0, -0x0C0, -0x090, -0x060, -0x030
    ], [
         0x000,  0x020,  0x040,  0x060,  0x080,  0x0A0,  0x0C0,  0x0E0,
         0x100,  0x120,  0x140,  0x160,  0x180,  0x1A0,  0x1C0,  0x1E0,
        -0x200, -0x1E0, -0x1C0, -0x1A0, -0x180, -0x160, -0x140, -0x120,
        -0x100, -0x0E0, -0x0C0, -0x0A0, -0x080, -0x060, -0x040, -0x020
    ], [
         0x000,  0x016,  0x02C,  0x042,  0x058,  0x06E,  0x084,  0x09A,
         0x0B0,  0x0C6,  0x0DC,  0x0F2,  0x108,  0x11E,  0x134,  0x14A,
        -0x160, -0x14A, -0x134, -0x11E, -0x108, -0x0F2, -0x0DC, -0x0C6,
        -0x0B0, -0x09A, -0x084, -0x06E, -0x058, -0x042, -0x02C, -0x016
    ], [
         0x000,  0x010,  0x020,  0x030,  0x040,  0x050,  0x060,  0x070,
         0x080,  0x090,  0x0A0,  0x0B0,  0x0C0,  0x0D0,  0x0E0,  0x0F0,
        -0x100, -0x0F0, -0x0E0, -0x0D0, -0x0C0, -0x0B0, -0x0A0, -0x090,
        -0x080, -0x070, -0x060, -0x050, -0x040, -0x030, -0x020, -0x010
    ], [
         0x00,  0x0B,  0x16,  0x21,  0x2C,  0x37,  0x42,  0x4D,
         0x58,  0x63,  0x6E,  0x79,  0x84,  0x8F,  0x9A,  0xA5,
        -0xB0, -0xA5, -0x9A, -0x8F, -0x84, -0x79, -0x6E, -0x63,
        -0x58, -0x4D, -0x42, -0x37, -0x2C, -0x21, -0x16, -0x0B
    ], [
         0x00,  0x08,  0x10,  0x18,  0x20,  0x28,  0x30,  0x38,
         0x40,  0x48,  0x50,  0x58,  0x60,  0x68,  0x70,  0x78,
        -0x80, -0x78, -0x70, -0x68, -0x60, -0x58, -0x50, -0x48,
        -0x40, -0x38, -0x30, -0x28, -0x20, -0x18, -0x10, -0x08
    ], [
         0x00,  0x06,  0x0C,  0x12,  0x18,  0x1E,  0x24,  0x2A,
         0x30,  0x36,  0x3C,  0x42,  0x48,  0x4E,  0x54,  0x5A,
        -0x60, -0x5A, -0x54, -0x4E, -0x48, -0x42, -0x3C, -0x36,
        -0x30, -0x2A, -0x24, -0x1E, -0x18, -0x12, -0x0C, -0x06
    ], [
         0x00,  0x04,  0x08,  0x0C,  0x10,  0x14,  0x18,  0x1C,
         0x20,  0x24,  0x28,  0x2C,  0x30,  0x34,  0x38,  0x3C,
        -0x40, -0x3C, -0x38, -0x34, -0x30, -0x2C, -0x28, -0x24,
        -0x20, -0x1C, -0x18, -0x14, -0x10, -0x0C, -0x08, -0x04
    ], [
         0x00,  0x02,  0x05,  0x08,  0x0B,  0x0D,  0x10,  0x13,
         0x16,  0x18,  0x1B,  0x1E,  0x21,  0x23,  0x26,  0x29,
        -0x2C, -0x2A, -0x27, -0x24, -0x21, -0x1F, -0x1C, -0x19,
        -0x16, -0x14, -0x11, -0x0E, -0x0B, -0x09, -0x06, -0x03
    ], [
         0x00,  0x01,  0x03,  0x05,  0x07,  0x08,  0x0A,  0x0C,
         0x0E,  0x0F,  0x11,  0x13,  0x15,  0x16,  0x18,  0x1A,
        -0x1C, -0x1B, -0x19, -0x17, -0x15, -0x14, -0x12, -0x10,
        -0x0E, -0x0D, -0x0B, -0x09, -0x07, -0x06, -0x04, -0x02
    ], [
         0x00,  0x01,  0x02,  0x03,  0x04,  0x05,  0x06,  0x07,
         0x08,  0x09,  0x0A,  0x0B,  0x0C,  0x0D,  0x0E,  0x0F,
        -0x10, -0x0F, -0x0E, -0x0D, -0x0C, -0x0B, -0x0A, -0x09,
        -0x08, -0x07, -0x06, -0x05, -0x04, -0x03, -0x02, -0x01
    ]
];
