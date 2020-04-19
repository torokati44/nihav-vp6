use nihav_core::codecs::*;
use nihav_core::io::bitreader::*;
use std::str::FromStr;
use std::sync::Arc;

const CODEC_SAMPLES: usize = 1152;

struct QMF {
    hist:       [f32; 1024],
    pos:        usize,
}

macro_rules! butterfly {
    (in; $src0: expr, $src1: expr, $dst0: expr, $dst1: expr, $scale: expr) => {
        $dst0 = $src0 + $src1;
        $dst1 = ($src0 - $src1) * $scale;
    };
    (rev; $a: expr, $b: expr) => {
        butterfly!(rev; $a, $b, std::f32::consts::FRAC_1_SQRT_2);
    };
    (rev; $a: expr, $b: expr, $scale: expr) => {
        let tmp = $a + $b;
        $b = ($a - $b) * $scale;
        $a = tmp;
    };
    (scal; $a: expr, $b: expr) => {
        butterfly!(scal; $a, $b, std::f32::consts::FRAC_1_SQRT_2);
    };
    (scal; $a: expr, $b: expr, $scale: expr) => {
        let tmp = $a + $b;
        $b = ($b - $a) * $scale;
        $a = tmp;
    };
}

macro_rules! postadd {
    ($tmp: expr, $a0: expr, $b0: expr, $c0: expr, $d0: expr, $a1: expr, $b1: expr, $c1: expr, $d1: expr) => {
        $tmp[$c0] += $tmp[$d0];
        $tmp[$c1] += $tmp[$d1];
        $tmp[$a1] += $tmp[$c1];
        $tmp[$c1] += $tmp[$b1];
        $tmp[$b1] += $tmp[$d1];
    }
}

fn dct32(src: &[f32; 32], dst: &mut [f32]) {
    let mut tmp = [0.0f32; 32];

    butterfly!(in; src[ 0], src[31], tmp[ 0], tmp[31], 0.50060299823519627);
    butterfly!(in; src[ 1], src[30], tmp[ 1], tmp[30], 0.50547095989754365);
    butterfly!(in; src[ 2], src[29], tmp[ 2], tmp[29], 0.51544730992262455);
    butterfly!(in; src[ 3], src[28], tmp[ 3], tmp[28], 0.53104259108978413);
    butterfly!(in; src[ 4], src[27], tmp[ 4], tmp[27], 0.55310389603444454);
    butterfly!(in; src[ 5], src[26], tmp[ 5], tmp[26], 0.58293496820613389);
    butterfly!(in; src[ 6], src[25], tmp[ 6], tmp[25], 0.62250412303566482);
    butterfly!(in; src[ 7], src[24], tmp[ 7], tmp[24], 0.67480834145500568);
    butterfly!(in; src[ 8], src[23], tmp[ 8], tmp[23], 0.74453627100229858);
    butterfly!(in; src[ 9], src[22], tmp[ 9], tmp[22], 0.83934964541552681);
    butterfly!(in; src[10], src[21], tmp[10], tmp[21], 0.97256823786196078);
    butterfly!(in; src[11], src[20], tmp[11], tmp[20], 1.1694399334328847);
    butterfly!(in; src[12], src[19], tmp[12], tmp[19], 1.4841646163141662);
    butterfly!(in; src[13], src[18], tmp[13], tmp[18], 2.0577810099534108);
    butterfly!(in; src[14], src[17], tmp[14], tmp[17], 3.407608418468719);
    butterfly!(in; src[15], src[16], tmp[15], tmp[16], 10.190008123548033);
    butterfly!(rev;  tmp[ 0], tmp[15], 0.50241928618815568);
    butterfly!(rev;  tmp[ 1], tmp[14], 0.52249861493968885);
    butterfly!(rev;  tmp[ 2], tmp[13], 0.56694403481635769);
    butterfly!(rev;  tmp[ 3], tmp[12], 0.64682178335999008);
    butterfly!(rev;  tmp[ 4], tmp[11], 0.7881546234512502);
    butterfly!(rev;  tmp[ 5], tmp[10], 1.0606776859903471);
    butterfly!(rev;  tmp[ 6], tmp[ 9], 1.7224470982383342);
    butterfly!(rev;  tmp[ 7], tmp[ 8], 5.1011486186891553);
    butterfly!(scal; tmp[16], tmp[31], 0.50241928618815568);
    butterfly!(scal; tmp[17], tmp[30], 0.52249861493968885);
    butterfly!(scal; tmp[18], tmp[29], 0.56694403481635769);
    butterfly!(scal; tmp[19], tmp[28], 0.64682178335999008);
    butterfly!(scal; tmp[20], tmp[27], 0.7881546234512502);
    butterfly!(scal; tmp[21], tmp[26], 1.0606776859903471);
    butterfly!(scal; tmp[22], tmp[25], 1.7224470982383342);
    butterfly!(scal; tmp[23], tmp[24], 5.1011486186891553);
    butterfly!(rev;  tmp[ 0], tmp[ 7], 0.50979557910415918);
    butterfly!(rev;  tmp[ 1], tmp[ 6], 0.60134488693504529);
    butterfly!(rev;  tmp[ 2], tmp[ 5], 0.89997622313641557);
    butterfly!(rev;  tmp[ 3], tmp[ 4], 2.5629154477415055);
    butterfly!(rev;  tmp[16], tmp[23], 0.50979557910415918);
    butterfly!(rev;  tmp[17], tmp[22], 0.60134488693504529);
    butterfly!(rev;  tmp[18], tmp[21], 0.89997622313641557);
    butterfly!(rev;  tmp[19], tmp[20], 2.5629154477415055);
    butterfly!(scal; tmp[ 8], tmp[15], 0.50979557910415918);
    butterfly!(scal; tmp[ 9], tmp[14], 0.60134488693504529);
    butterfly!(scal; tmp[10], tmp[13], 0.89997622313641557);
    butterfly!(scal; tmp[11], tmp[12], 2.5629154477415055);
    butterfly!(scal; tmp[24], tmp[31], 0.50979557910415918);
    butterfly!(scal; tmp[25], tmp[30], 0.60134488693504529);
    butterfly!(scal; tmp[26], tmp[29], 0.89997622313641557);
    butterfly!(scal; tmp[27], tmp[28], 2.5629154477415055);
    butterfly!(rev;  tmp[ 0], tmp[ 3], 0.54119610014619701);
    butterfly!(rev;  tmp[ 1], tmp[ 2], 1.3065629648763764);
    butterfly!(rev;  tmp[ 8], tmp[11], 0.54119610014619701);
    butterfly!(rev;  tmp[ 9], tmp[10], 1.3065629648763764);
    butterfly!(rev;  tmp[16], tmp[19], 0.54119610014619701);
    butterfly!(rev;  tmp[17], tmp[18], 1.3065629648763764);
    butterfly!(rev;  tmp[24], tmp[27], 0.54119610014619701);
    butterfly!(rev;  tmp[25], tmp[26], 1.3065629648763764);
    butterfly!(scal; tmp[ 4], tmp[ 7], 0.54119610014619701);
    butterfly!(scal; tmp[ 5], tmp[ 6], 1.3065629648763764);
    butterfly!(scal; tmp[12], tmp[15], 0.54119610014619701);
    butterfly!(scal; tmp[13], tmp[14], 1.3065629648763764);
    butterfly!(scal; tmp[20], tmp[23], 0.54119610014619701);
    butterfly!(scal; tmp[21], tmp[22], 1.3065629648763764);
    butterfly!(scal; tmp[28], tmp[31], 0.54119610014619701);
    butterfly!(scal; tmp[29], tmp[30], 1.3065629648763764);
    butterfly!(rev;  tmp[ 0], tmp[ 1]);
    butterfly!(rev;  tmp[ 4], tmp[ 5]);
    butterfly!(rev;  tmp[ 8], tmp[ 9]);
    butterfly!(rev;  tmp[12], tmp[13]);
    butterfly!(rev;  tmp[16], tmp[17]);
    butterfly!(rev;  tmp[20], tmp[21]);
    butterfly!(rev;  tmp[24], tmp[25]);
    butterfly!(rev;  tmp[28], tmp[29]);
    butterfly!(scal; tmp[ 2], tmp[ 3]);
    butterfly!(scal; tmp[ 6], tmp[ 7]);
    butterfly!(scal; tmp[10], tmp[11]);
    butterfly!(scal; tmp[14], tmp[15]);
    butterfly!(scal; tmp[18], tmp[19]);
    butterfly!(scal; tmp[22], tmp[23]);
    butterfly!(scal; tmp[26], tmp[27]);
    butterfly!(scal; tmp[30], tmp[31]);

    postadd!(tmp, 0,  1,  2,  3,  4,  5,  6,  7);
    postadd!(tmp, 8,  9, 10, 11, 12, 13, 14, 15);
    postadd!(tmp,16, 17, 18, 19, 20, 21, 22, 23);
    postadd!(tmp,24, 25, 26, 27, 28, 29, 30, 31);

    dst[ 0] = tmp[0];
    dst[16] = tmp[1];
    dst[ 8] = tmp[2];
    dst[24] = tmp[3];
    dst[ 4] = tmp[4];
    dst[20] = tmp[5];
    dst[12] = tmp[6];
    dst[28] = tmp[7];

    dst[ 2] = tmp[ 8] + tmp[12];
    dst[18] = tmp[ 9] + tmp[13];
    dst[10] = tmp[10] + tmp[14];
    dst[26] = tmp[11] + tmp[15];

    dst[ 6] = tmp[12] + tmp[10];
    dst[22] = tmp[13] + tmp[11];

    dst[14] = tmp[14] + tmp[ 9];

    dst[30] = tmp[15];

    tmp[24] += tmp[28];
    tmp[28] += tmp[26];
    tmp[26] += tmp[30];
    tmp[30] += tmp[25];
    tmp[25] += tmp[29];
    tmp[29] += tmp[27];
    tmp[27] += tmp[31];

    dst[ 1] = tmp[16] + tmp[24];
    dst[17] = tmp[17] + tmp[25];
    dst[ 9] = tmp[18] + tmp[26];
    dst[25] = tmp[19] + tmp[27];
    dst[ 5] = tmp[20] + tmp[28];
    dst[21] = tmp[21] + tmp[29];
    dst[13] = tmp[22] + tmp[30];
    dst[29] = tmp[23] + tmp[31];

    dst[ 3] = tmp[24] + tmp[20];
    dst[19] = tmp[25] + tmp[21];
    dst[11] = tmp[26] + tmp[22];
    dst[27] = tmp[27] + tmp[23];

    dst[ 7] = tmp[28] + tmp[18];
    dst[23] = tmp[29] + tmp[19];

    dst[15] = tmp[30] + tmp[17];

    dst[31] = tmp[31];
}

impl QMF {
    fn new() -> Self {
        Self {
            hist:   [0.0; 1024],
            pos:    0,
        }
    }
    fn synth(&mut self, src: &[f32; 32], dst: &mut [f32]) {
        self.pos = self.pos.wrapping_sub(32) & 0x1FF;
        dct32(src, &mut self.hist[self.pos..][..32]);

        for i in 0..16 {
            let mut acc = 0.0;
            for j in (0..512).step_by(64) {
                acc += QMF_WINDOW[i + j]      * self.hist[(j + 16      + i + self.pos) & 0x1FF];
                acc -= QMF_WINDOW[i + j + 32] * self.hist[(j + 16 + 32 - i + self.pos) & 0x1FF];
            }
            dst[i] = acc;
        }
        let mut acc = 0.0;
        for j in (0..512).step_by(64) {
            acc -= QMF_WINDOW[j + 32 + 16] * self.hist[(j + 32 + self.pos) & 0x1FF];
        }
        dst[16] = acc;
        for i in 17..32 {
            let mut acc = 0.0;
            for j in (0..512).step_by(64) {
                acc -= QMF_WINDOW[i + j]      * self.hist[(j + 32 + 16 - i + self.pos) & 0x1FF];
                acc -= QMF_WINDOW[i + j + 32] * self.hist[(j + 32 - 16 + i + self.pos) & 0x1FF];
            }
            dst[i] = acc;
        }
    }
}

struct LHDecoder {
    ainfo:      NAAudioInfo,
    info:       Arc<NACodecInfo>,
    chmap:      NAChannelMap,

    bitalloc:   [[u8; 32]; 3],
    scf_select: [u8; 32],
    scales:     [[u8; 32]; 3],
    samples:    [[f32; 32]; 36],

    bitpos:     u32,

    qmf:        QMF,
}

impl LHDecoder {
    fn new() -> Self {
        Self {
            ainfo:      NAAudioInfo::new(22050, 1, SND_F32P_FORMAT, CODEC_SAMPLES),
            info:       NACodecInfo::new_dummy(),
            chmap:      NAChannelMap::new(),

            bitalloc:   [[0; 32]; 3],
            scf_select: [0; 32],
            scales:     [[0; 32]; 3],
            samples:    [[0.0; 32]; 36],

            bitpos:     0,

            qmf:        QMF::new(),
        }
    }
    fn unpack_bitalloc(&mut self, br: &mut BitReader) -> DecoderResult<()> {
        for i in 0..3 {
            for sb in 0..32 {
                self.bitalloc[i][sb] = br.read(BITALLOC_INFO[sb])? as u8;
            }
        }
        Ok(())
    }
    fn unpack_scales(&mut self, br: &mut BitReader) -> DecoderResult<()> {
        for sb in 0..32 {
            if (self.bitalloc[0][sb] | self.bitalloc[1][sb] | self.bitalloc[2][sb]) != 0 {
                self.scf_select[sb] = br.read(2)? as u8;
            } else {
                self.scf_select[sb] = 0;
            }
        }

        self.scales = [[0; 32]; 3];
        for sb in 0..32 {
            let ba0 = self.bitalloc[0][sb];
            let ba1 = self.bitalloc[1][sb];
            let ba2 = self.bitalloc[2][sb];
            if (ba0 | ba1 | ba2) == 0 {
                continue;
            }
            match self.scf_select[sb] {
                0 => {
                    for j in 0..3 {
                        if self.bitalloc[j][sb] != 0 {
                            self.scales[j][sb] = br.read(6)? as u8;
                        }
                    }
                },
                1 => {
                    if (ba0 | ba1) != 0 {
                        let scale = br.read(6)? as u8;
                        self.scales[0][sb] = scale;
                        self.scales[1][sb] = scale;
                    }
                    if ba2 != 0 {
                        self.scales[2][sb] = br.read(6)? as u8;
                    }
                },
                2 => {
                    let scale = br.read(6)? as u8;
                    self.scales[0][sb] = scale;
                    self.scales[1][sb] = scale;
                    self.scales[2][sb] = scale;
                },
                _ => {
                    if ba0 != 0 {
                        self.scales[0][sb] = br.read(6)? as u8;
                    }
                    if (ba1 | ba2) != 0 {
                        let scale = br.read(6)? as u8;
                        self.scales[1][sb] = scale;
                        self.scales[2][sb] = scale;
                    }
                },
            };
        }
        Ok(())
    }
    fn unpack_samples(&mut self, br: &mut BitReader) -> DecoderResult<()> {
        for grp in 0..3 {
            for gr in 0..4 {
                for sb in 0..32 {
                    let set = (grp * 4 + gr) * 3;
                    if self.bitalloc[grp][sb] == 0 {
                        self.samples[set + 0][sb] = 0.0;
                        self.samples[set + 1][sb] = 0.0;
                        self.samples[set + 2][sb] = 0.0;
                        continue;
                    }
                    let idx = sb * 4 + (self.bitalloc[grp][sb] as usize);
                    let bits = GROUP_BITS[idx];
                    let sf = SCALEFACTORS[self.scales[grp][sb] as usize];
                    if GROUP_INFO[idx] == 1 {
                        let radix = (1 << bits) - 1;
                        let val0 = br.read(bits)? as usize;
                        let val1 = br.read(bits)? as usize;
                        let val2 = br.read(bits)? as usize;
                        self.samples[set + 0][sb] = Self::dequant(val0, idx, radix) * sf;
                        self.samples[set + 1][sb] = Self::dequant(val1, idx, radix) * sf;
                        self.samples[set + 2][sb] = Self::dequant(val2, idx, radix) * sf;
                    } else {
                        let radix = GROUP_RADIX[idx] as usize;
                        let val = br.read(bits)? as usize;
                        let val0 = val % radix;
                        let val1 = (val / radix) % radix;
                        let val2 = val / radix / radix;
                        self.samples[set + 0][sb] = Self::dequant(val0, idx, radix) * sf;
                        self.samples[set + 1][sb] = Self::dequant(val1, idx, radix) * sf;
                        self.samples[set + 2][sb] = Self::dequant(val2, idx, radix) * sf;
                    }
                }
            }
        }
        Ok(())
    }
    fn dequant(val: usize, idx: usize, radix: usize) -> f32 {
        let qval = match radix {
                3  => QUANTS3[val],
                5  => QUANTS5[val],
                7  => QUANTS7[val],
                15 => QUANTS15[val],
                63 => QUANTS63[val],
                _  => unreachable!(),
            };
        let bias_idx = QUANT_BIAS_MAP[idx] as usize;
        (qval + QUANT_BIAS[bias_idx]) / QUANT_RANGE[bias_idx]
    }
}

impl NADecoder for LHDecoder {
    fn init(&mut self, _supp: &mut NADecoderSupport, info: NACodecInfoRef) -> DecoderResult<()> {
        if let NACodecTypeInfo::Audio(ainfo) = info.get_properties() {
            self.ainfo = NAAudioInfo::new(ainfo.get_sample_rate(), 1, SND_F32P_FORMAT, CODEC_SAMPLES);
            self.info = info.replace_info(NACodecTypeInfo::Audio(self.ainfo.clone()));
            self.chmap = NAChannelMap::from_str("C").unwrap();
            Ok(())
        } else {
            Err(DecoderError::InvalidData)
        }
    }
    fn decode(&mut self, _supp: &mut NADecoderSupport, pkt: &NAPacket) -> DecoderResult<NAFrameRef> {
        let info = pkt.get_stream().get_info();
        if let NACodecTypeInfo::Audio(_) = info.get_properties() {
            let pktbuf = pkt.get_buffer();

            let mut daudio = Vec::with_capacity(CODEC_SAMPLES);

            let mut br = BitReader::new(pktbuf.as_slice(), BitReaderMode::BE);
            br.skip(self.bitpos)?;

            while br.left() >= 8 {
                self.unpack_bitalloc(&mut br)?;
                self.unpack_scales(&mut br)?;
                self.unpack_samples(&mut br)?;

                let mut samp_buf = [0.0f32; 32];
                for set in 0..36 {
                    self.qmf.synth(&self.samples[set], &mut samp_buf);
                    daudio.extend_from_slice(&samp_buf);
                }
            }

            self.bitpos = (br.tell() as u32) & 7;

            let abuf = alloc_audio_buffer(self.ainfo, daudio.len(), self.chmap.clone())?;
            let mut adata = abuf.get_abuf_f32().unwrap();
            let buf = adata.get_data_mut().unwrap();
            (&mut buf[..daudio.len()]).copy_from_slice(daudio.as_slice());

            let mut frm = NAFrame::new_from_pkt(pkt, self.info.clone(), abuf);
            frm.set_duration(Some(CODEC_SAMPLES as u64));
            frm.set_keyframe(true);
            Ok(frm.into_ref())
        } else {
            Err(DecoderError::Bug)
        }
    }
    fn flush(&mut self) {
        self.qmf = QMF::new();
        self.bitpos = 0;
    }
}

pub fn get_decoder() -> Box<dyn NADecoder + Send> {
    Box::new(LHDecoder::new())
}

const BITALLOC_INFO: [u8; 32] = [
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1
];

const GROUP_BITS: [u8; 128] = [
    0, 3, 4, 6, 0, 3, 4, 6, 0, 3, 4, 6, 0, 3, 4, 6,
    0, 3, 4, 6, 0, 3, 4, 6, 0, 5, 7, 4, 0, 5, 7, 4,
    0, 5, 7, 4, 0, 5, 7, 4, 0, 5, 7, 4, 0, 5, 7, 4,
    0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0,
    0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0,
    0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0,
    0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0,
    0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0
];
const GROUP_INFO: [u8; 128] = [
    0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1,
    0, 1, 1, 1, 0, 1, 1, 1, 0, 3, 3, 1, 0, 3, 3, 1,
    0, 3, 3, 1, 0, 3, 3, 1, 0, 3, 3, 1, 0, 3, 3, 1,
    0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0,
    0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0,
    0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0,
    0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0,
    0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0
];
const GROUP_RADIX: [u8; 128] = [
    0,  7, 15, 63,  0,  7, 15, 63,  0,  7, 15, 63,  0,  7, 15, 63,
    0,  7, 15, 63,  0,  7, 15, 63,  0,  3,  5, 15,  0,  3,  5, 15,
    0,  3,  5, 15,  0,  3,  5, 15,  0,  3,  5, 15,  0,  3,  5, 15,
    0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,
    0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,
    0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,
    0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,
    0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0,  0,  3,  0,  0
];

const QUANT_BIAS_MAP: [u8; 128] = [
    0, 2, 4, 6, 0, 2, 4, 6, 0, 2, 4, 6, 0, 2, 4, 6,
    0, 2, 4, 6, 0, 2, 4, 6, 0, 0, 1, 4, 0, 0, 1, 4,
    0, 0, 1, 4, 0, 0, 1, 4, 0, 0, 1, 4, 0, 0, 1, 4,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
];
const QUANT_BIAS: [f32; 17] = [
    0.5, 0.5, 0.25, 0.5, 0.125, 0.0625, 0.03125, 0.015625,
    0.0078125, 0.00390625, 0.001953125, 0.0009765625, 0.00048828125,
    0.00024414062, 0.00012207031, 0.000061035164, 0.000030517582
];
const QUANT_RANGE: [f32; 17] = [
    0.75, 0.625, 0.875, 0.5625, 0.9375, 0.96875, 0.984375,
    0.9921875, 0.99609375, 0.99804688, 0.99902344, 0.99951172,
    0.99975586, 0.99987793, 0.99993896, 0.99996948, 0.99998474
];
const SCALEFACTORS: [f32; 64] = [
    2.0, 1.587401, 1.2599211, 1.0, 0.79370052, 0.62996054,
    0.5, 0.39685026, 0.31498027, 0.25, 0.19842513, 0.15749013,
    0.125, 0.099212565, 0.078745067, 0.0625, 0.049606282, 0.039372534,
    0.03125, 0.024803141, 0.019686267, 0.015625, 0.012401571, 0.0098431334,
    0.0078125, 0.0062007853, 0.0049215667, 0.00390625, 0.0031003926, 0.0024607833,
    0.001953125, 0.0015501963, 0.0012303917, 0.0009765625, 0.00077509816, 0.00061519584,
    0.00048828125, 0.00038754908, 0.00030759792, 0.00024414062, 0.00019377454, 0.00015379896,
    0.00012207031, 0.00009688727, 0.00007689948, 0.000061035156,
    0.000048443635, 0.00003844974, 0.000030517578, 0.000024221818,
    0.00001922487, 0.000015258789, 0.000012110909, 0.000009612435,
    0.0000076293945, 0.0000060554544, 0.0000048062175, 0.0000038146973,
    0.0000030277272, 0.0000024031087, 0.0000019073486, 0.0000015138636,
    0.0000012015544, 9.9999997e-21
];

const QUANTS3: [f32; 4] = [ -1.0, -0.5, 0.0, 0.5 ];
const QUANTS5: [f32; 6] = [ -1.0, -0.75, -0.5, -0.25, 0.0, 0.25 ];
const QUANTS7: [f32; 8] = [ -1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75 ];
const QUANTS15: [f32; 16] = [
    -1.0, -0.875, -0.75, -0.625, -0.5, -0.375, -0.25, -0.125,
     0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875 ];
const QUANTS63: [f32; 64] = [
    -1.0,  -0.96875, -0.9375, -0.90625, -0.875, -0.84375, -0.8125, -0.78125,
    -0.75, -0.71875, -0.6875, -0.65625, -0.625, -0.59375, -0.5625, -0.53125,
    -0.5,  -0.46875, -0.4375, -0.40625, -0.375, -0.34375, -0.3125, -0.28125,
    -0.25, -0.21875, -0.1875, -0.15625, -0.125, -0.09375, -0.0625, -0.03125,
     0.0,   0.03125,  0.0625,  0.09375,  0.125,  0.15625,  0.1875,  0.21875,
     0.25,  0.28125,  0.3125,  0.34375,  0.375,  0.40625,  0.4375,  0.46875,
     0.5,   0.53125,  0.5625,  0.59375,  0.625,  0.65625,  0.6875,  0.71875,
     0.75,  0.78125,  0.8125,  0.84375,  0.875,  0.90625,  0.9375,  0.96875 ];

const QMF_WINDOW: [f32; 512] = [
  0.000000000, -0.000015259, -0.000015259, -0.000015259,
 -0.000015259, -0.000015259, -0.000015259, -0.000030518,
 -0.000030518, -0.000030518, -0.000030518, -0.000045776,
 -0.000045776, -0.000061035, -0.000061035, -0.000076294,
 -0.000076294, -0.000091553, -0.000106812, -0.000106812,
 -0.000122070, -0.000137329, -0.000152588, -0.000167847,
 -0.000198364, -0.000213623, -0.000244141, -0.000259399,
 -0.000289917, -0.000320435, -0.000366211, -0.000396729,
 -0.000442505, -0.000473022, -0.000534058, -0.000579834,
 -0.000625610, -0.000686646, -0.000747681, -0.000808716,
 -0.000885010, -0.000961304, -0.001037598, -0.001113892,
 -0.001205444, -0.001296997, -0.001388550, -0.001480103,
 -0.001586914, -0.001693726, -0.001785278, -0.001907349,
 -0.002014160, -0.002120972, -0.002243042, -0.002349854,
 -0.002456665, -0.002578735, -0.002685547, -0.002792358,
 -0.002899170, -0.002990723, -0.003082275, -0.003173828,
  0.003250122,  0.003326416,  0.003387451,  0.003433228,
  0.003463745,  0.003479004,  0.003479004,  0.003463745,
  0.003417969,  0.003372192,  0.003280640,  0.003173828,
  0.003051758,  0.002883911,  0.002700806,  0.002487183,
  0.002227783,  0.001937866,  0.001617432,  0.001266479,
  0.000869751,  0.000442505, -0.000030518, -0.000549316,
 -0.001098633, -0.001693726, -0.002334595, -0.003005981,
 -0.003723145, -0.004486084, -0.005294800, -0.006118774,
 -0.007003784, -0.007919312, -0.008865356, -0.009841919,
 -0.010848999, -0.011886597, -0.012939453, -0.014022827,
 -0.015121460, -0.016235352, -0.017349243, -0.018463135,
 -0.019577026, -0.020690918, -0.021789551, -0.022857666,
 -0.023910522, -0.024932861, -0.025909424, -0.026840210,
 -0.027725220, -0.028533936, -0.029281616, -0.029937744,
 -0.030532837, -0.031005859, -0.031387329, -0.031661987,
 -0.031814575, -0.031845093, -0.031738281, -0.031478882,
  0.031082153,  0.030517578,  0.029785156,  0.028884888,
  0.027801514,  0.026535034,  0.025085449,  0.023422241,
  0.021575928,  0.019531250,  0.017257690,  0.014801025,
  0.012115479,  0.009231567,  0.006134033,  0.002822876,
 -0.000686646, -0.004394531, -0.008316040, -0.012420654,
 -0.016708374, -0.021179199, -0.025817871, -0.030609131,
 -0.035552979, -0.040634155, -0.045837402, -0.051132202,
 -0.056533813, -0.061996460, -0.067520142, -0.073059082,
 -0.078628540, -0.084182739, -0.089706421, -0.095169067,
 -0.100540161, -0.105819702, -0.110946655, -0.115921021,
 -0.120697021, -0.125259399, -0.129562378, -0.133590698,
 -0.137298584, -0.140670776, -0.143676758, -0.146255493,
 -0.148422241, -0.150115967, -0.151306152, -0.151962280,
 -0.152069092, -0.151596069, -0.150497437, -0.148773193,
 -0.146362305, -0.143264771, -0.139450073, -0.134887695,
 -0.129577637, -0.123474121, -0.116577148, -0.108856201,
  0.100311279,  0.090927124,  0.080688477,  0.069595337,
  0.057617187,  0.044784546,  0.031082153,  0.016510010,
  0.001068115, -0.015228271, -0.032379150, -0.050354004,
 -0.069168091, -0.088775635, -0.109161377, -0.130310059,
 -0.152206421, -0.174789429, -0.198059082, -0.221984863,
 -0.246505737, -0.271591187, -0.297210693, -0.323318481,
 -0.349868774, -0.376800537, -0.404083252, -0.431655884,
 -0.459472656, -0.487472534, -0.515609741, -0.543823242,
 -0.572036743, -0.600219727, -0.628295898, -0.656219482,
 -0.683914185, -0.711318970, -0.738372803, -0.765029907,
 -0.791213989, -0.816864014, -0.841949463, -0.866363525,
 -0.890090942, -0.913055420, -0.935195923, -0.956481934,
 -0.976852417, -0.996246338, -1.014617920, -1.031936646,
 -1.048156738, -1.063217163, -1.077117920, -1.089782715,
 -1.101211548, -1.111373901, -1.120223999, -1.127746582,
 -1.133926392, -1.138763428, -1.142211914, -1.144287109,
  1.144989014,  1.144287109,  1.142211914,  1.138763428,
  1.133926392,  1.127746582,  1.120223999,  1.111373901,
  1.101211548,  1.089782715,  1.077117920,  1.063217163,
  1.048156738,  1.031936646,  1.014617920,  0.996246338,
  0.976852417,  0.956481934,  0.935195923,  0.913055420,
  0.890090942,  0.866363525,  0.841949463,  0.816864014,
  0.791213989,  0.765029907,  0.738372803,  0.711318970,
  0.683914185,  0.656219482,  0.628295898,  0.600219727,
  0.572036743,  0.543823242,  0.515609741,  0.487472534,
  0.459472656,  0.431655884,  0.404083252,  0.376800537,
  0.349868774,  0.323318481,  0.297210693,  0.271591187,
  0.246505737,  0.221984863,  0.198059082,  0.174789429,
  0.152206421,  0.130310059,  0.109161377,  0.088775635,
  0.069168091,  0.050354004,  0.032379150,  0.015228271,
 -0.001068115, -0.016510010, -0.031082153, -0.044784546,
 -0.057617187, -0.069595337, -0.080688477, -0.090927124,
  0.100311279,  0.108856201,  0.116577148,  0.123474121,
  0.129577637,  0.134887695,  0.139450073,  0.143264771,
  0.146362305,  0.148773193,  0.150497437,  0.151596069,
  0.152069092,  0.151962280,  0.151306152,  0.150115967,
  0.148422241,  0.146255493,  0.143676758,  0.140670776,
  0.137298584,  0.133590698,  0.129562378,  0.125259399,
  0.120697021,  0.115921021,  0.110946655,  0.105819702,
  0.100540161,  0.095169067,  0.089706421,  0.084182739,
  0.078628540,  0.073059082,  0.067520142,  0.061996460,
  0.056533813,  0.051132202,  0.045837402,  0.040634155,
  0.035552979,  0.030609131,  0.025817871,  0.021179199,
  0.016708374,  0.012420654,  0.008316040,  0.004394531,
  0.000686646, -0.002822876, -0.006134033, -0.009231567,
 -0.012115479, -0.014801025, -0.017257690, -0.019531250,
 -0.021575928, -0.023422241, -0.025085449, -0.026535034,
 -0.027801514, -0.028884888, -0.029785156, -0.030517578,
  0.031082153,  0.031478882,  0.031738281,  0.031845093,
  0.031814575,  0.031661987,  0.031387329,  0.031005859,
  0.030532837,  0.029937744,  0.029281616,  0.028533936,
  0.027725220,  0.026840210,  0.025909424,  0.024932861,
  0.023910522,  0.022857666,  0.021789551,  0.020690918,
  0.019577026,  0.018463135,  0.017349243,  0.016235352,
  0.015121460,  0.014022827,  0.012939453,  0.011886597,
  0.010848999,  0.009841919,  0.008865356,  0.007919312,
  0.007003784,  0.006118774,  0.005294800,  0.004486084,
  0.003723145,  0.003005981,  0.002334595,  0.001693726,
  0.001098633,  0.000549316,  0.000030518, -0.000442505,
 -0.000869751, -0.001266479, -0.001617432, -0.001937866,
 -0.002227783, -0.002487183, -0.002700806, -0.002883911,
 -0.003051758, -0.003173828, -0.003280640, -0.003372192,
 -0.003417969, -0.003463745, -0.003479004, -0.003479004,
 -0.003463745, -0.003433228, -0.003387451, -0.003326416,
  0.003250122,  0.003173828,  0.003082275,  0.002990723,
  0.002899170,  0.002792358,  0.002685547,  0.002578735,
  0.002456665,  0.002349854,  0.002243042,  0.002120972,
  0.002014160,  0.001907349,  0.001785278,  0.001693726,
  0.001586914,  0.001480103,  0.001388550,  0.001296997,
  0.001205444,  0.001113892,  0.001037598,  0.000961304,
  0.000885010,  0.000808716,  0.000747681,  0.000686646,
  0.000625610,  0.000579834,  0.000534058,  0.000473022,
  0.000442505,  0.000396729,  0.000366211,  0.000320435,
  0.000289917,  0.000259399,  0.000244141,  0.000213623,
  0.000198364,  0.000167847,  0.000152588,  0.000137329,
  0.000122070,  0.000106812,  0.000106812,  0.000091553,
  0.000076294,  0.000076294,  0.000061035,  0.000061035,
  0.000045776,  0.000045776,  0.000030518,  0.000030518,
  0.000030518,  0.000030518,  0.000015259,  0.000015259,
  0.000015259,  0.000015259,  0.000015259,  0.000015259,
];
