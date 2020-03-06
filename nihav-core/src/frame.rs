//! Packets and decoded frames functionality.
use std::cmp::max;
//use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
pub use crate::formats::*;
pub use crate::refs::*;

/// Audio stream information.
#[allow(dead_code)]
#[derive(Clone,Copy,PartialEq)]
pub struct NAAudioInfo {
    /// Sample rate.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels:    u8,
    /// Audio sample format.
    pub format:      NASoniton,
    /// Length of one audio block in samples.
    pub block_len:   usize,
}

impl NAAudioInfo {
    /// Constructs a new `NAAudioInfo` instance.
    pub fn new(sr: u32, ch: u8, fmt: NASoniton, bl: usize) -> Self {
        NAAudioInfo { sample_rate: sr, channels: ch, format: fmt, block_len: bl }
    }
    /// Returns audio sample rate.
    pub fn get_sample_rate(&self) -> u32 { self.sample_rate }
    /// Returns the number of channels.
    pub fn get_channels(&self) -> u8 { self.channels }
    /// Returns sample format.
    pub fn get_format(&self) -> NASoniton { self.format }
    /// Returns one audio block duration in samples.
    pub fn get_block_len(&self) -> usize { self.block_len }
}

impl fmt::Display for NAAudioInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} Hz, {} ch", self.sample_rate, self.channels)
    }
}

/// Video stream information.
#[allow(dead_code)]
#[derive(Clone,Copy,PartialEq)]
pub struct NAVideoInfo {
    /// Picture width.
    pub width:      usize,
    /// Picture height.
    pub height:     usize,
    /// Picture is stored downside up.
    pub flipped:    bool,
    /// Picture pixel format.
    pub format:     NAPixelFormaton,
}

impl NAVideoInfo {
    /// Constructs a new `NAVideoInfo` instance.
    pub fn new(w: usize, h: usize, flip: bool, fmt: NAPixelFormaton) -> Self {
        NAVideoInfo { width: w, height: h, flipped: flip, format: fmt }
    }
    /// Returns picture width.
    pub fn get_width(&self)  -> usize { self.width as usize }
    /// Returns picture height.
    pub fn get_height(&self) -> usize { self.height as usize }
    /// Returns picture orientation.
    pub fn is_flipped(&self) -> bool { self.flipped }
    /// Returns picture pixel format.
    pub fn get_format(&self) -> NAPixelFormaton { self.format }
    /// Sets new picture width.
    pub fn set_width(&mut self, w: usize)  { self.width  = w; }
    /// Sets new picture height.
    pub fn set_height(&mut self, h: usize) { self.height = h; }
}

impl fmt::Display for NAVideoInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

/// A list of possible stream information types.
#[derive(Clone,Copy,PartialEq)]
pub enum NACodecTypeInfo {
    /// No codec present.
    None,
    /// Audio codec information.
    Audio(NAAudioInfo),
    /// Video codec information.
    Video(NAVideoInfo),
}

impl NACodecTypeInfo {
    /// Returns video stream information.
    pub fn get_video_info(&self) -> Option<NAVideoInfo> {
        match *self {
            NACodecTypeInfo::Video(vinfo) => Some(vinfo),
            _ => None,
        }
    }
    /// Returns audio stream information.
    pub fn get_audio_info(&self) -> Option<NAAudioInfo> {
        match *self {
            NACodecTypeInfo::Audio(ainfo) => Some(ainfo),
            _ => None,
        }
    }
    /// Reports whether the current stream is video stream.
    pub fn is_video(&self) -> bool {
        match *self {
            NACodecTypeInfo::Video(_) => true,
            _ => false,
        }
    }
    /// Reports whether the current stream is audio stream.
    pub fn is_audio(&self) -> bool {
        match *self {
            NACodecTypeInfo::Audio(_) => true,
            _ => false,
        }
    }
}

impl fmt::Display for NACodecTypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ret = match *self {
            NACodecTypeInfo::None       => "".to_string(),
            NACodecTypeInfo::Audio(fmt) => format!("{}", fmt),
            NACodecTypeInfo::Video(fmt) => format!("{}", fmt),
        };
        write!(f, "{}", ret)
    }
}

/// Decoded video frame.
///
/// NihAV frames are stored in native type (8/16/32-bit elements) inside a single buffer.
/// In case of image with several components those components are stored sequentially and can be accessed in the buffer starting at corresponding component offset.
#[derive(Clone)]
pub struct NAVideoBuffer<T> {
    info:    NAVideoInfo,
    data:    NABufferRef<Vec<T>>,
    offs:    Vec<usize>,
    strides: Vec<usize>,
}

impl<T: Clone> NAVideoBuffer<T> {
    /// Returns the component offset (0 for all unavailable offsets).
    pub fn get_offset(&self, idx: usize) -> usize {
        if idx >= self.offs.len() { 0 }
        else { self.offs[idx] }
    }
    /// Returns picture info.
    pub fn get_info(&self) -> NAVideoInfo { self.info }
    /// Returns an immutable reference to the data.
    pub fn get_data(&self) -> &Vec<T> { self.data.as_ref() }
    /// Returns a mutable reference to the data.
    pub fn get_data_mut(&mut self) -> Option<&mut Vec<T>> { self.data.as_mut() }
    /// Returns the number of components in picture format.
    pub fn get_num_components(&self) -> usize { self.offs.len() }
    /// Creates a copy of current `NAVideoBuffer`.
    pub fn copy_buffer(&mut self) -> Self {
        let mut data: Vec<T> = Vec::with_capacity(self.data.len());
        data.clone_from(self.data.as_ref());
        let mut offs: Vec<usize> = Vec::with_capacity(self.offs.len());
        offs.clone_from(&self.offs);
        let mut strides: Vec<usize> = Vec::with_capacity(self.strides.len());
        strides.clone_from(&self.strides);
        NAVideoBuffer { info: self.info, data: NABufferRef::new(data), offs, strides }
    }
    /// Returns stride (distance between subsequent lines) for the requested component.
    pub fn get_stride(&self, idx: usize) -> usize {
        if idx >= self.strides.len() { return 0; }
        self.strides[idx]
    }
    /// Returns requested component dimensions.
    pub fn get_dimensions(&self, idx: usize) -> (usize, usize) {
        get_plane_size(&self.info, idx)
    }
    /// Converts current instance into buffer reference.
    pub fn into_ref(self) -> NABufferRef<Self> {
        NABufferRef::new(self)
    }
}

/// A specialised type for reference-counted `NAVideoBuffer`.
pub type NAVideoBufferRef<T> = NABufferRef<NAVideoBuffer<T>>;

/// Decoded audio frame.
///
/// NihAV frames are stored in native type (8/16/32-bit elements) inside a single buffer.
/// In case of planar audio samples for each channel are stored sequentially and can be accessed in the buffer starting at corresponding channel offset.
#[derive(Clone)]
pub struct NAAudioBuffer<T> {
    info:   NAAudioInfo,
    data:   NABufferRef<Vec<T>>,
    offs:   Vec<usize>,
    stride: usize,
    step:   usize,
    chmap:  NAChannelMap,
    len:    usize,
}

impl<T: Clone> NAAudioBuffer<T> {
    /// Returns the start position of requested channel data.
    pub fn get_offset(&self, idx: usize) -> usize {
        if idx >= self.offs.len() { 0 }
        else { self.offs[idx] }
    }
    /// Returns the distance between the start of one channel and the next one.
    pub fn get_stride(&self) -> usize { self.stride }
    /// Returns the distance between the samples in one channel.
    pub fn get_step(&self) -> usize { self.step }
    /// Returns audio format information.
    pub fn get_info(&self) -> NAAudioInfo { self.info }
    /// Returns channel map.
    pub fn get_chmap(&self) -> &NAChannelMap { &self.chmap }
    /// Returns an immutable reference to the data.
    pub fn get_data(&self) -> &Vec<T> { self.data.as_ref() }
    /// Returns a mutable reference to the data.
    pub fn get_data_mut(&mut self) -> Option<&mut Vec<T>> { self.data.as_mut() }
    /// Clones current `NAAudioBuffer` into a new one.
    pub fn copy_buffer(&mut self) -> Self {
        let mut data: Vec<T> = Vec::with_capacity(self.data.len());
        data.clone_from(self.data.as_ref());
        let mut offs: Vec<usize> = Vec::with_capacity(self.offs.len());
        offs.clone_from(&self.offs);
        NAAudioBuffer { info: self.info, data: NABufferRef::new(data), offs, chmap: self.get_chmap().clone(), len: self.len, stride: self.stride, step: self.step }
    }
    /// Return the length of frame in samples.
    pub fn get_length(&self) -> usize { self.len }
}

impl NAAudioBuffer<u8> {
    /// Constructs a new `NAAudioBuffer` instance.
    pub fn new_from_buf(info: NAAudioInfo, data: NABufferRef<Vec<u8>>, chmap: NAChannelMap) -> Self {
        let len = data.len();
        NAAudioBuffer { info, data, chmap, offs: Vec::new(), len, stride: 0, step: 0 }
    }
}

/// A list of possible decoded frame types.
#[derive(Clone)]
pub enum NABufferType {
    /// 8-bit video buffer.
    Video      (NAVideoBufferRef<u8>),
    /// 16-bit video buffer (i.e. every component or packed pixel fits into 16 bits).
    Video16    (NAVideoBufferRef<u16>),
    /// 32-bit video buffer (i.e. every component or packed pixel fits into 32 bits).
    Video32    (NAVideoBufferRef<u32>),
    /// Packed video buffer.
    VideoPacked(NAVideoBufferRef<u8>),
    /// Audio buffer with 8-bit unsigned integer audio.
    AudioU8    (NAAudioBuffer<u8>),
    /// Audio buffer with 16-bit signed integer audio.
    AudioI16   (NAAudioBuffer<i16>),
    /// Audio buffer with 32-bit signed integer audio.
    AudioI32   (NAAudioBuffer<i32>),
    /// Audio buffer with 32-bit floating point audio.
    AudioF32   (NAAudioBuffer<f32>),
    /// Packed audio buffer.
    AudioPacked(NAAudioBuffer<u8>),
    /// Buffer with generic data (e.g. subtitles).
    Data       (NABufferRef<Vec<u8>>),
    /// No data present.
    None,
}

impl NABufferType {
    /// Returns the offset to the requested component or channel.
    pub fn get_offset(&self, idx: usize) -> usize {
        match *self {
            NABufferType::Video(ref vb)       => vb.get_offset(idx),
            NABufferType::Video16(ref vb)     => vb.get_offset(idx),
            NABufferType::Video32(ref vb)     => vb.get_offset(idx),
            NABufferType::VideoPacked(ref vb) => vb.get_offset(idx),
            NABufferType::AudioU8(ref ab)     => ab.get_offset(idx),
            NABufferType::AudioI16(ref ab)    => ab.get_offset(idx),
            NABufferType::AudioF32(ref ab)    => ab.get_offset(idx),
            NABufferType::AudioPacked(ref ab) => ab.get_offset(idx),
            _ => 0,
        }
    }
    /// Returns information for video frames.
    pub fn get_video_info(&self) -> Option<NAVideoInfo> {
        match *self {
            NABufferType::Video(ref vb)       => Some(vb.get_info()),
            NABufferType::Video16(ref vb)     => Some(vb.get_info()),
            NABufferType::Video32(ref vb)     => Some(vb.get_info()),
            NABufferType::VideoPacked(ref vb) => Some(vb.get_info()),
            _ => None,
        }
    }
    /// Returns reference to 8-bit (or packed) video buffer.
    pub fn get_vbuf(&self) -> Option<NAVideoBufferRef<u8>> {
        match *self {
            NABufferType::Video(ref vb)       => Some(vb.clone()),
            NABufferType::VideoPacked(ref vb) => Some(vb.clone()),
            _ => None,
        }
    }
    /// Returns reference to 16-bit video buffer.
    pub fn get_vbuf16(&self) -> Option<NAVideoBufferRef<u16>> {
        match *self {
            NABufferType::Video16(ref vb)     => Some(vb.clone()),
            _ => None,
        }
    }
    /// Returns reference to 32-bit video buffer.
    pub fn get_vbuf32(&self) -> Option<NAVideoBufferRef<u32>> {
        match *self {
            NABufferType::Video32(ref vb)     => Some(vb.clone()),
            _ => None,
        }
    }
    /// Returns information for audio frames.
    pub fn get_audio_info(&self) -> Option<NAAudioInfo> {
        match *self {
            NABufferType::AudioU8(ref ab)     => Some(ab.get_info()),
            NABufferType::AudioI16(ref ab)    => Some(ab.get_info()),
            NABufferType::AudioI32(ref ab)    => Some(ab.get_info()),
            NABufferType::AudioF32(ref ab)    => Some(ab.get_info()),
            NABufferType::AudioPacked(ref ab) => Some(ab.get_info()),
            _ => None,
        }
    }
    /// Returns audio channel map.
    pub fn get_chmap(&self) -> Option<&NAChannelMap> {
        match *self {
            NABufferType::AudioU8(ref ab)     => Some(ab.get_chmap()),
            NABufferType::AudioI16(ref ab)    => Some(ab.get_chmap()),
            NABufferType::AudioI32(ref ab)    => Some(ab.get_chmap()),
            NABufferType::AudioF32(ref ab)    => Some(ab.get_chmap()),
            NABufferType::AudioPacked(ref ab) => Some(ab.get_chmap()),
            _ => None,
        }
    }
    /// Returns audio frame duration in samples.
    pub fn get_audio_length(&self) -> usize {
        match *self {
            NABufferType::AudioU8(ref ab)     => ab.get_length(),
            NABufferType::AudioI16(ref ab)    => ab.get_length(),
            NABufferType::AudioI32(ref ab)    => ab.get_length(),
            NABufferType::AudioF32(ref ab)    => ab.get_length(),
            NABufferType::AudioPacked(ref ab) => ab.get_length(),
            _ => 0,
        }
    }
    /// Returns the distance between starts of two channels.
    pub fn get_audio_stride(&self) -> usize {
        match *self {
            NABufferType::AudioU8(ref ab)     => ab.get_stride(),
            NABufferType::AudioI16(ref ab)    => ab.get_stride(),
            NABufferType::AudioI32(ref ab)    => ab.get_stride(),
            NABufferType::AudioF32(ref ab)    => ab.get_stride(),
            NABufferType::AudioPacked(ref ab) => ab.get_stride(),
            _ => 0,
        }
    }
    /// Returns the distance between two samples in one channel.
    pub fn get_audio_step(&self) -> usize {
        match *self {
            NABufferType::AudioU8(ref ab)     => ab.get_step(),
            NABufferType::AudioI16(ref ab)    => ab.get_step(),
            NABufferType::AudioI32(ref ab)    => ab.get_step(),
            NABufferType::AudioF32(ref ab)    => ab.get_step(),
            NABufferType::AudioPacked(ref ab) => ab.get_step(),
            _ => 0,
        }
    }
    /// Returns reference to 8-bit (or packed) audio buffer.
    pub fn get_abuf_u8(&self) -> Option<NAAudioBuffer<u8>> {
        match *self {
            NABufferType::AudioU8(ref ab) => Some(ab.clone()),
            NABufferType::AudioPacked(ref ab) => Some(ab.clone()),
            _ => None,
        }
    }
    /// Returns reference to 16-bit audio buffer.
    pub fn get_abuf_i16(&self) -> Option<NAAudioBuffer<i16>> {
        match *self {
            NABufferType::AudioI16(ref ab) => Some(ab.clone()),
            _ => None,
        }
    }
    /// Returns reference to 32-bit integer audio buffer.
    pub fn get_abuf_i32(&self) -> Option<NAAudioBuffer<i32>> {
        match *self {
            NABufferType::AudioI32(ref ab) => Some(ab.clone()),
            _ => None,
        }
    }
    /// Returns reference to 32-bit floating point audio buffer.
    pub fn get_abuf_f32(&self) -> Option<NAAudioBuffer<f32>> {
        match *self {
            NABufferType::AudioF32(ref ab) => Some(ab.clone()),
            _ => None,
        }
    }
}

const NA_SIMPLE_VFRAME_COMPONENTS: usize = 4;
/// Simplified decoded frame data.
pub struct NASimpleVideoFrame<'a, T: Copy> {
    /// Widths of each picture component.
    pub width:      [usize; NA_SIMPLE_VFRAME_COMPONENTS],
    /// Heights of each picture component.
    pub height:     [usize; NA_SIMPLE_VFRAME_COMPONENTS],
    /// Orientation (upside-down or downside-up) flag.
    pub flip:       bool,
    /// Strides for each component.
    pub stride:     [usize; NA_SIMPLE_VFRAME_COMPONENTS],
    /// Start of each component.
    pub offset:     [usize; NA_SIMPLE_VFRAME_COMPONENTS],
    /// Number of components.
    pub components: usize,
    /// Pointer to the picture pixel data.
    pub data:       &'a mut [T],
}

impl<'a, T:Copy> NASimpleVideoFrame<'a, T> {
    /// Constructs a new instance of `NASimpleVideoFrame` from `NAVideoBuffer`.
    pub fn from_video_buf(vbuf: &'a mut NAVideoBuffer<T>) -> Option<Self> {
        let vinfo = vbuf.get_info();
        let components = vinfo.format.components as usize;
        if components > NA_SIMPLE_VFRAME_COMPONENTS {
            return None;
        }
        let mut w: [usize; NA_SIMPLE_VFRAME_COMPONENTS] = [0; NA_SIMPLE_VFRAME_COMPONENTS];
        let mut h: [usize; NA_SIMPLE_VFRAME_COMPONENTS] = [0; NA_SIMPLE_VFRAME_COMPONENTS];
        let mut s: [usize; NA_SIMPLE_VFRAME_COMPONENTS] = [0; NA_SIMPLE_VFRAME_COMPONENTS];
        let mut o: [usize; NA_SIMPLE_VFRAME_COMPONENTS] = [0; NA_SIMPLE_VFRAME_COMPONENTS];
        for comp in 0..components {
            let (width, height) = vbuf.get_dimensions(comp);
            w[comp] = width;
            h[comp] = height;
            s[comp] = vbuf.get_stride(comp);
            o[comp] = vbuf.get_offset(comp);
        }
        let flip = vinfo.flipped;
        Some(NASimpleVideoFrame {
            width:  w,
            height: h,
            flip,
            stride: s,
            offset: o,
            components,
            data: vbuf.data.as_mut_slice(),
            })
    }
}

/// A list of possible frame allocator errors.
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum AllocatorError {
    /// Requested picture dimensions are too large.
    TooLargeDimensions,
    /// Invalid input format.
    FormatError,
}

/// Constructs a new video buffer with requested format.
///
/// `align` is power of two alignment for image. E.g. the value of 5 means that frame dimensions will be padded to be multiple of 32.
pub fn alloc_video_buffer(vinfo: NAVideoInfo, align: u8) -> Result<NABufferType, AllocatorError> {
    let fmt = &vinfo.format;
    let mut new_size: usize = 0;
    let mut offs:    Vec<usize> = Vec::new();
    let mut strides: Vec<usize> = Vec::new();

    for i in 0..fmt.get_num_comp() {
        if fmt.get_chromaton(i) == None { return Err(AllocatorError::FormatError); }
    }

    let align_mod = ((1 << align) as usize) - 1;
    let width  = ((vinfo.width  as usize) + align_mod) & !align_mod;
    let height = ((vinfo.height as usize) + align_mod) & !align_mod;
    let mut max_depth = 0;
    let mut all_packed = true;
    let mut all_bytealigned = true;
    for i in 0..fmt.get_num_comp() {
        let ochr = fmt.get_chromaton(i);
        if ochr.is_none() { continue; }
        let chr = ochr.unwrap();
        if !chr.is_packed() {
            all_packed = false;
        } else if ((chr.get_shift() + chr.get_depth()) & 7) != 0 {
            all_bytealigned = false;
        }
        max_depth = max(max_depth, chr.get_depth());
    }
    let unfit_elem_size = match fmt.get_elem_size() {
            2 | 4 => false,
            _ => true,
        };

//todo semi-packed like NV12
    if fmt.is_paletted() {
//todo various-sized palettes?
        let stride = vinfo.get_format().get_chromaton(0).unwrap().get_linesize(width);
        let pic_sz = stride.checked_mul(height);
        if pic_sz == None { return Err(AllocatorError::TooLargeDimensions); }
        let pal_size = 256 * (fmt.get_elem_size() as usize);
        let new_size = pic_sz.unwrap().checked_add(pal_size);
        if new_size == None { return Err(AllocatorError::TooLargeDimensions); }
        offs.push(0);
        offs.push(stride * height);
        strides.push(stride);
        let data: Vec<u8> = vec![0; new_size.unwrap()];
        let buf: NAVideoBuffer<u8> = NAVideoBuffer { data: NABufferRef::new(data), info: vinfo, offs, strides };
        Ok(NABufferType::Video(buf.into_ref()))
    } else if !all_packed {
        for i in 0..fmt.get_num_comp() {
            let ochr = fmt.get_chromaton(i);
            if ochr.is_none() { continue; }
            let chr = ochr.unwrap();
            offs.push(new_size as usize);
            let stride = chr.get_linesize(width);
            let cur_h = chr.get_height(height);
            let cur_sz = stride.checked_mul(cur_h);
            if cur_sz == None { return Err(AllocatorError::TooLargeDimensions); }
            let new_sz = new_size.checked_add(cur_sz.unwrap());
            if new_sz == None { return Err(AllocatorError::TooLargeDimensions); }
            new_size = new_sz.unwrap();
            strides.push(stride);
        }
        if max_depth <= 8 {
            let data: Vec<u8> = vec![0; new_size];
            let buf: NAVideoBuffer<u8> = NAVideoBuffer { data: NABufferRef::new(data), info: vinfo, offs, strides };
            Ok(NABufferType::Video(buf.into_ref()))
        } else if max_depth <= 16 {
            let data: Vec<u16> = vec![0; new_size];
            let buf: NAVideoBuffer<u16> = NAVideoBuffer { data: NABufferRef::new(data), info: vinfo, offs, strides };
            Ok(NABufferType::Video16(buf.into_ref()))
        } else {
            let data: Vec<u32> = vec![0; new_size];
            let buf: NAVideoBuffer<u32> = NAVideoBuffer { data: NABufferRef::new(data), info: vinfo, offs, strides };
            Ok(NABufferType::Video32(buf.into_ref()))
        }
    } else if all_bytealigned || unfit_elem_size {
        let elem_sz = fmt.get_elem_size();
        let line_sz = width.checked_mul(elem_sz as usize);
        if line_sz == None { return Err(AllocatorError::TooLargeDimensions); }
        let new_sz = line_sz.unwrap().checked_mul(height);
        if new_sz == None { return Err(AllocatorError::TooLargeDimensions); }
        new_size = new_sz.unwrap();
        let data: Vec<u8> = vec![0; new_size];
        strides.push(line_sz.unwrap());
        let buf: NAVideoBuffer<u8> = NAVideoBuffer { data: NABufferRef::new(data), info: vinfo, offs, strides };
        Ok(NABufferType::VideoPacked(buf.into_ref()))
    } else {
        let elem_sz = fmt.get_elem_size();
        let new_sz = width.checked_mul(height);
        if new_sz == None { return Err(AllocatorError::TooLargeDimensions); }
        new_size = new_sz.unwrap();
        match elem_sz {
            2 => {
                    let data: Vec<u16> = vec![0; new_size];
                    strides.push(width);
                    let buf: NAVideoBuffer<u16> = NAVideoBuffer { data: NABufferRef::new(data), info: vinfo, offs, strides };
                    Ok(NABufferType::Video16(buf.into_ref()))
                },
            4 => {
                    let data: Vec<u32> = vec![0; new_size];
                    strides.push(width);
                    let buf: NAVideoBuffer<u32> = NAVideoBuffer { data: NABufferRef::new(data), info: vinfo, offs, strides };
                    Ok(NABufferType::Video32(buf.into_ref()))
                },
            _ => unreachable!(),
        }
    }
}

/// Constructs a new audio buffer for the requested format and length.
#[allow(clippy::collapsible_if)]
pub fn alloc_audio_buffer(ainfo: NAAudioInfo, nsamples: usize, chmap: NAChannelMap) -> Result<NABufferType, AllocatorError> {
    let mut offs: Vec<usize> = Vec::new();
    if ainfo.format.is_planar() || ((ainfo.format.get_bits() % 8) == 0) {
        let len = nsamples.checked_mul(ainfo.channels as usize);
        if len == None { return Err(AllocatorError::TooLargeDimensions); }
        let length = len.unwrap();
        let stride;
        let step;
        if ainfo.format.is_planar() {
            stride = nsamples;
            step   = 1;
            for i in 0..ainfo.channels {
                offs.push((i as usize) * stride);
            }
        } else {
            stride = 1;
            step   = ainfo.channels as usize;
            for i in 0..ainfo.channels {
                offs.push(i as usize);
            }
        }
        if ainfo.format.is_float() {
            if ainfo.format.get_bits() == 32 {
                let data: Vec<f32> = vec![0.0; length];
                let buf: NAAudioBuffer<f32> = NAAudioBuffer { data: NABufferRef::new(data), info: ainfo, offs, chmap, len: nsamples, stride, step };
                Ok(NABufferType::AudioF32(buf))
            } else {
                Err(AllocatorError::TooLargeDimensions)
            }
        } else {
            if ainfo.format.get_bits() == 8 && !ainfo.format.is_signed() {
                let data: Vec<u8> = vec![0; length];
                let buf: NAAudioBuffer<u8> = NAAudioBuffer { data: NABufferRef::new(data), info: ainfo, offs, chmap, len: nsamples, stride, step };
                Ok(NABufferType::AudioU8(buf))
            } else if ainfo.format.get_bits() == 16 && ainfo.format.is_signed() {
                let data: Vec<i16> = vec![0; length];
                let buf: NAAudioBuffer<i16> = NAAudioBuffer { data: NABufferRef::new(data), info: ainfo, offs, chmap, len: nsamples, stride, step };
                Ok(NABufferType::AudioI16(buf))
            } else {
                Err(AllocatorError::TooLargeDimensions)
            }
        }
    } else {
        let len = nsamples.checked_mul(ainfo.channels as usize);
        if len == None { return Err(AllocatorError::TooLargeDimensions); }
        let length = ainfo.format.get_audio_size(len.unwrap() as u64);
        let data: Vec<u8> = vec![0; length];
        let buf: NAAudioBuffer<u8> = NAAudioBuffer { data: NABufferRef::new(data), info: ainfo, offs, chmap, len: nsamples, stride: 0, step: 0 };
        Ok(NABufferType::AudioPacked(buf))
    }
}

/// Constructs a new buffer for generic data.
pub fn alloc_data_buffer(size: usize) -> Result<NABufferType, AllocatorError> {
    let data: Vec<u8> = vec![0; size];
    let buf: NABufferRef<Vec<u8>> = NABufferRef::new(data);
    Ok(NABufferType::Data(buf))
}

/// Creates a clone of current buffer.
pub fn copy_buffer(buf: NABufferType) -> NABufferType {
    buf.clone()
}

/// Video frame pool.
///
/// This structure allows codec to effectively reuse old frames instead of allocating and de-allocating frames every time.
/// Caller can also reserve some frames for its own purposes e.g. display queue.
pub struct NAVideoBufferPool<T:Copy> {
    pool:       Vec<NAVideoBufferRef<T>>,
    max_len:    usize,
    add_len:    usize,
}

impl<T:Copy> NAVideoBufferPool<T> {
    /// Constructs a new `NAVideoBufferPool` instance.
    pub fn new(max_len: usize) -> Self {
        Self {
            pool:       Vec::with_capacity(max_len),
            max_len,
            add_len: 0,
        }
    }
    /// Sets the number of buffers reserved for the user.
    pub fn set_dec_bufs(&mut self, add_len: usize) {
        self.add_len = add_len;
    }
    /// Returns an unused buffer from the pool.
    pub fn get_free(&mut self) -> Option<NAVideoBufferRef<T>> {
        for e in self.pool.iter() {
            if e.get_num_refs() == 1 {
                return Some(e.clone());
            }
        }
        None
    }
    /// Clones provided frame data into a free pool frame.
    pub fn get_copy(&mut self, rbuf: &NAVideoBufferRef<T>) -> Option<NAVideoBufferRef<T>> {
        let mut dbuf = self.get_free()?;
        dbuf.data.copy_from_slice(&rbuf.data);
        Some(dbuf)
    }
    /// Clears the pool from all frames.
    pub fn reset(&mut self) {
        self.pool.truncate(0);
    }
}

impl NAVideoBufferPool<u8> {
    /// Allocates the target amount of video frames using [`alloc_video_buffer`].
    ///
    /// [`alloc_video_buffer`]: ./fn.alloc_video_buffer.html
    pub fn prealloc_video(&mut self, vinfo: NAVideoInfo, align: u8) -> Result<(), AllocatorError> {
        let nbufs = self.max_len + self.add_len - self.pool.len();
        for _ in 0..nbufs {
            let vbuf = alloc_video_buffer(vinfo, align)?;
            if let NABufferType::Video(buf) = vbuf {
                self.pool.push(buf);
            } else if let NABufferType::VideoPacked(buf) = vbuf {
                self.pool.push(buf);
            } else {
                return Err(AllocatorError::FormatError);
            }
        }
        Ok(())
    }
}

impl NAVideoBufferPool<u16> {
    /// Allocates the target amount of video frames using [`alloc_video_buffer`].
    ///
    /// [`alloc_video_buffer`]: ./fn.alloc_video_buffer.html
    pub fn prealloc_video(&mut self, vinfo: NAVideoInfo, align: u8) -> Result<(), AllocatorError> {
        let nbufs = self.max_len + self.add_len - self.pool.len();
        for _ in 0..nbufs {
            let vbuf = alloc_video_buffer(vinfo, align)?;
            if let NABufferType::Video16(buf) = vbuf {
                self.pool.push(buf);
            } else {
                return Err(AllocatorError::FormatError);
            }
        }
        Ok(())
    }
}

impl NAVideoBufferPool<u32> {
    /// Allocates the target amount of video frames using [`alloc_video_buffer`].
    ///
    /// [`alloc_video_buffer`]: ./fn.alloc_video_buffer.html
    pub fn prealloc_video(&mut self, vinfo: NAVideoInfo, align: u8) -> Result<(), AllocatorError> {
        let nbufs = self.max_len + self.add_len - self.pool.len();
        for _ in 0..nbufs {
            let vbuf = alloc_video_buffer(vinfo, align)?;
            if let NABufferType::Video32(buf) = vbuf {
                self.pool.push(buf);
            } else {
                return Err(AllocatorError::FormatError);
            }
        }
        Ok(())
    }
}

/// Information about codec contained in a stream.
#[allow(dead_code)]
#[derive(Clone)]
pub struct NACodecInfo {
    name:       &'static str,
    properties: NACodecTypeInfo,
    extradata:  Option<Arc<Vec<u8>>>,
}

/// A specialised type for reference-counted `NACodecInfo`.
pub type NACodecInfoRef = Arc<NACodecInfo>;

impl NACodecInfo {
    /// Constructs a new instance of `NACodecInfo`.
    pub fn new(name: &'static str, p: NACodecTypeInfo, edata: Option<Vec<u8>>) -> Self {
        let extradata = match edata {
            None => None,
            Some(vec) => Some(Arc::new(vec)),
        };
        NACodecInfo { name, properties: p, extradata }
    }
    /// Constructs a new reference-counted instance of `NACodecInfo`.
    pub fn new_ref(name: &'static str, p: NACodecTypeInfo, edata: Option<Arc<Vec<u8>>>) -> Self {
        NACodecInfo { name, properties: p, extradata: edata }
    }
    /// Converts current instance into a reference-counted one.
    pub fn into_ref(self) -> NACodecInfoRef { Arc::new(self) }
    /// Returns codec information.
    pub fn get_properties(&self) -> NACodecTypeInfo { self.properties }
    /// Returns additional initialisation data required by the codec.
    pub fn get_extradata(&self) -> Option<Arc<Vec<u8>>> {
        if let Some(ref vec) = self.extradata { return Some(vec.clone()); }
        None
    }
    /// Returns codec name.
    pub fn get_name(&self) -> &'static str { self.name }
    /// Reports whether it is a video codec.
    pub fn is_video(&self) -> bool {
        if let NACodecTypeInfo::Video(_) = self.properties { return true; }
        false
    }
    /// Reports whether it is an audio codec.
    pub fn is_audio(&self) -> bool {
        if let NACodecTypeInfo::Audio(_) = self.properties { return true; }
        false
    }
    /// Constructs a new empty reference-counted instance of `NACodecInfo`.
    pub fn new_dummy() -> Arc<Self> {
        Arc::new(DUMMY_CODEC_INFO)
    }
    /// Updates codec infomation.
    pub fn replace_info(&self, p: NACodecTypeInfo) -> Arc<Self> {
        Arc::new(NACodecInfo { name: self.name, properties: p, extradata: self.extradata.clone() })
    }
}

impl Default for NACodecInfo {
    fn default() -> Self { DUMMY_CODEC_INFO }
}

impl fmt::Display for NACodecInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let edata = match self.extradata.clone() {
            None => "no extradata".to_string(),
            Some(v) => format!("{} byte(s) of extradata", v.len()),
        };
        write!(f, "{}: {} {}", self.name, self.properties, edata)
    }
}

/// Default empty codec information.
pub const DUMMY_CODEC_INFO: NACodecInfo = NACodecInfo {
                                name: "none",
                                properties: NACodecTypeInfo::None,
                                extradata: None };

/// A list of accepted option values.
#[derive(Debug,Clone)]
pub enum NAValue {
    /// Empty value.
    None,
    /// Integer value.
    Int(i32),
    /// Long integer value.
    Long(i64),
    /// String value.
    String(String),
    /// Binary data value.
    Data(Arc<Vec<u8>>),
}

/// A list of recognized frame types.
#[derive(Debug,Clone,Copy,PartialEq)]
#[allow(dead_code)]
pub enum FrameType {
    /// Intra frame type.
    I,
    /// Inter frame type.
    P,
    /// Bidirectionally predicted frame.
    B,
    /// Skip frame.
    ///
    /// When such frame is encountered then last frame should be used again if it is needed.
    Skip,
    /// Some other frame type.
    Other,
}

impl fmt::Display for FrameType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FrameType::I => write!(f, "I"),
            FrameType::P => write!(f, "P"),
            FrameType::B => write!(f, "B"),
            FrameType::Skip => write!(f, "skip"),
            FrameType::Other => write!(f, "x"),
        }
    }
}

/// Timestamp information.
#[derive(Debug,Clone,Copy)]
pub struct NATimeInfo {
    /// Presentation timestamp.
    pub pts:            Option<u64>,
    /// Decode timestamp.
    pub dts:            Option<u64>,
    /// Duration (in timebase units).
    pub duration:       Option<u64>,
    /// Timebase numerator.
    pub tb_num:         u32,
    /// Timebase denominator.
    pub tb_den:         u32,
}

impl NATimeInfo {
    /// Constructs a new `NATimeInfo` instance.
    pub fn new(pts: Option<u64>, dts: Option<u64>, duration: Option<u64>, tb_num: u32, tb_den: u32) -> Self {
        NATimeInfo { pts, dts, duration, tb_num, tb_den }
    }
    /// Returns presentation timestamp.
    pub fn get_pts(&self) -> Option<u64> { self.pts }
    /// Returns decoding timestamp.
    pub fn get_dts(&self) -> Option<u64> { self.dts }
    /// Returns duration.
    pub fn get_duration(&self) -> Option<u64> { self.duration }
    /// Sets new presentation timestamp.
    pub fn set_pts(&mut self, pts: Option<u64>) { self.pts = pts; }
    /// Sets new decoding timestamp.
    pub fn set_dts(&mut self, dts: Option<u64>) { self.dts = dts; }
    /// Sets new duration.
    pub fn set_duration(&mut self, dur: Option<u64>) { self.duration = dur; }

    /// Converts time in given scale into timestamp in given base.
    pub fn time_to_ts(time: u64, base: u64, tb_num: u32, tb_den: u32) -> u64 {
        let tb_num = tb_num as u64;
        let tb_den = tb_den as u64;
        let tmp = time.checked_mul(tb_num);
        if let Some(tmp) = tmp {
            tmp / base / tb_den
        } else {
            let tmp = time.checked_mul(tb_num);
            if let Some(tmp) = tmp {
                tmp / base / tb_den
            } else {
                let coarse = time / base;
                let tmp = coarse.checked_mul(tb_num);
                if let Some(tmp) = tmp {
                    tmp / tb_den
                } else {
                    (coarse / tb_den) * tb_num
                }
            }
        }
    }
    /// Converts timestamp in given base into time in given scale.
    pub fn ts_to_time(ts: u64, base: u64, tb_num: u32, tb_den: u32) -> u64 {
        let tb_num = tb_num as u64;
        let tb_den = tb_den as u64;
        let tmp = ts.checked_mul(base);
        if let Some(tmp) = tmp {
            let tmp2 = tmp.checked_mul(tb_num);
            if let Some(tmp2) = tmp2 {
                tmp2 / tb_den
            } else {
                (tmp / tb_den) * tb_num
            }
        } else {
            let tmp = ts.checked_mul(tb_num);
            if let Some(tmp) = tmp {
                (tmp / tb_den) * base
            } else {
                (ts / tb_den) * base * tb_num
            }
        }
    }
}

/// Decoded frame information.
#[allow(dead_code)]
#[derive(Clone)]
pub struct NAFrame {
    /// Frame timestamp.
    pub ts:             NATimeInfo,
    /// Frame ID.
    pub id:             i64,
        buffer:         NABufferType,
        info:           NACodecInfoRef,
    /// Frame type.
    pub frame_type:     FrameType,
    /// Keyframe flag.
    pub key:            bool,
//        options:        HashMap<String, NAValue>,
}

/// A specialised type for reference-counted `NAFrame`.
pub type NAFrameRef = Arc<NAFrame>;

fn get_plane_size(info: &NAVideoInfo, idx: usize) -> (usize, usize) {
    let chromaton = info.get_format().get_chromaton(idx);
    if chromaton.is_none() { return (0, 0); }
    let (hs, vs) = chromaton.unwrap().get_subsampling();
    let w = (info.get_width()  + ((1 << hs) - 1)) >> hs;
    let h = (info.get_height() + ((1 << vs) - 1)) >> vs;
    (w, h)
}

impl NAFrame {
    /// Constructs a new `NAFrame` instance.
    pub fn new(ts:             NATimeInfo,
               ftype:          FrameType,
               keyframe:       bool,
               info:           NACodecInfoRef,
               /*options:        HashMap<String, NAValue>,*/
               buffer:         NABufferType) -> Self {
        NAFrame { ts, id: 0, buffer, info, frame_type: ftype, key: keyframe/*, options*/ }
    }
    /// Returns frame format information.
    pub fn get_info(&self) -> NACodecInfoRef { self.info.clone() }
    /// Returns frame type.
    pub fn get_frame_type(&self) -> FrameType { self.frame_type }
    /// Reports whether the frame is a keyframe.
    pub fn is_keyframe(&self) -> bool { self.key }
    /// Sets new frame type.
    pub fn set_frame_type(&mut self, ftype: FrameType) { self.frame_type = ftype; }
    /// Sets keyframe flag.
    pub fn set_keyframe(&mut self, key: bool) { self.key = key; }
    /// Returns frame timestamp.
    pub fn get_time_information(&self) -> NATimeInfo { self.ts }
    /// Returns frame presentation time.
    pub fn get_pts(&self) -> Option<u64> { self.ts.get_pts() }
    /// Returns frame decoding time.
    pub fn get_dts(&self) -> Option<u64> { self.ts.get_dts() }
    /// Returns picture ID.
    pub fn get_id(&self) -> i64 { self.id }
    /// Returns frame display duration.
    pub fn get_duration(&self) -> Option<u64> { self.ts.get_duration() }
    /// Sets new presentation timestamp.
    pub fn set_pts(&mut self, pts: Option<u64>) { self.ts.set_pts(pts); }
    /// Sets new decoding timestamp.
    pub fn set_dts(&mut self, dts: Option<u64>) { self.ts.set_dts(dts); }
    /// Sets new picture ID.
    pub fn set_id(&mut self, id: i64) { self.id = id; }
    /// Sets new duration.
    pub fn set_duration(&mut self, dur: Option<u64>) { self.ts.set_duration(dur); }

    /// Returns a reference to the frame data.
    pub fn get_buffer(&self) -> NABufferType { self.buffer.clone() }

    /// Converts current instance into a reference-counted one.
    pub fn into_ref(self) -> NAFrameRef { Arc::new(self) }

    /// Creates new frame with metadata from `NAPacket`.
    pub fn new_from_pkt(pkt: &NAPacket, info: NACodecInfoRef, buf: NABufferType) -> NAFrame {
        NAFrame::new(pkt.ts, FrameType::Other, pkt.keyframe, info, /*HashMap::new(),*/ buf)
    }
}

impl fmt::Display for NAFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut ostr = format!("frame type {}", self.frame_type);
        if let Some(pts) = self.ts.pts { ostr = format!("{} pts {}", ostr, pts); }
        if let Some(dts) = self.ts.dts { ostr = format!("{} dts {}", ostr, dts); }
        if let Some(dur) = self.ts.duration { ostr = format!("{} duration {}", ostr, dur); }
        if self.key { ostr = format!("{} kf", ostr); }
        write!(f, "[{}]", ostr)
    }
}

/// A list of possible stream types.
#[derive(Debug,Clone,Copy,PartialEq)]
#[allow(dead_code)]
pub enum StreamType {
    /// Video stream.
    Video,
    /// Audio stream.
    Audio,
    /// Subtitles.
    Subtitles,
    /// Any data stream (or might be an unrecognized audio/video stream).
    Data,
    /// Nonexistent stream.
    None,
}

impl fmt::Display for StreamType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StreamType::Video => write!(f, "Video"),
            StreamType::Audio => write!(f, "Audio"),
            StreamType::Subtitles => write!(f, "Subtitles"),
            StreamType::Data => write!(f, "Data"),
            StreamType::None => write!(f, "-"),
        }
    }
}

/// Stream data.
#[allow(dead_code)]
#[derive(Clone)]
pub struct NAStream {
        media_type:     StreamType,
    /// Stream ID.
    pub id:             u32,
        num:            usize,
        info:           NACodecInfoRef,
    /// Timebase numerator.
    pub tb_num:         u32,
    /// Timebase denominator.
    pub tb_den:         u32,
}

/// A specialised reference-counted `NAStream` type.
pub type NAStreamRef = Arc<NAStream>;

/// Downscales the timebase by its greatest common denominator.
pub fn reduce_timebase(tb_num: u32, tb_den: u32) -> (u32, u32) {
    if tb_num == 0 { return (tb_num, tb_den); }
    if (tb_den % tb_num) == 0 { return (1, tb_den / tb_num); }

    let mut a = tb_num;
    let mut b = tb_den;

    while a != b {
        if a > b { a -= b; }
        else if b > a { b -= a; }
    }

    (tb_num / a, tb_den / a)
}

impl NAStream {
    /// Constructs a new `NAStream` instance.
    pub fn new(mt: StreamType, id: u32, info: NACodecInfo, tb_num: u32, tb_den: u32) -> Self {
        let (n, d) = reduce_timebase(tb_num, tb_den);
        NAStream { media_type: mt, id, num: 0, info: info.into_ref(), tb_num: n, tb_den: d }
    }
    /// Returns stream id.
    pub fn get_id(&self) -> u32 { self.id }
    /// Returns stream type.
    pub fn get_media_type(&self) -> StreamType { self.media_type }
    /// Returns stream number assigned by demuxer.
    pub fn get_num(&self) -> usize { self.num }
    /// Sets stream number.
    pub fn set_num(&mut self, num: usize) { self.num = num; }
    /// Returns codec information.
    pub fn get_info(&self) -> NACodecInfoRef { self.info.clone() }
    /// Returns stream timebase.
    pub fn get_timebase(&self) -> (u32, u32) { (self.tb_num, self.tb_den) }
    /// Sets new stream timebase.
    pub fn set_timebase(&mut self, tb_num: u32, tb_den: u32) {
        let (n, d) = reduce_timebase(tb_num, tb_den);
        self.tb_num = n;
        self.tb_den = d;
    }
    /// Converts current instance into a reference-counted one.
    pub fn into_ref(self) -> NAStreamRef { Arc::new(self) }
}

impl fmt::Display for NAStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}#{} @ {}/{} - {})", self.media_type, self.id, self.tb_num, self.tb_den, self.info.get_properties())
    }
}

/// Packet with compressed data.
#[allow(dead_code)]
pub struct NAPacket {
        stream:         NAStreamRef,
    /// Packet timestamp.
    pub ts:             NATimeInfo,
        buffer:         NABufferRef<Vec<u8>>,
    /// Keyframe flag.
    pub keyframe:       bool,
//    options:        HashMap<String, NAValue<'a>>,
}

impl NAPacket {
    /// Constructs a new `NAPacket` instance.
    pub fn new(str: NAStreamRef, ts: NATimeInfo, kf: bool, vec: Vec<u8>) -> Self {
//        let mut vec: Vec<u8> = Vec::new();
//        vec.resize(size, 0);
        NAPacket { stream: str, ts, keyframe: kf, buffer: NABufferRef::new(vec) }
    }
    /// Returns information about the stream packet belongs to.
    pub fn get_stream(&self) -> NAStreamRef { self.stream.clone() }
    /// Returns packet timestamp.
    pub fn get_time_information(&self) -> NATimeInfo { self.ts }
    /// Returns packet presentation timestamp.
    pub fn get_pts(&self) -> Option<u64> { self.ts.get_pts() }
    /// Returns packet decoding timestamp.
    pub fn get_dts(&self) -> Option<u64> { self.ts.get_dts() }
    /// Returns packet duration.
    pub fn get_duration(&self) -> Option<u64> { self.ts.get_duration() }
    /// Reports whether this is a keyframe packet.
    pub fn is_keyframe(&self) -> bool { self.keyframe }
    /// Returns a reference to packet data.
    pub fn get_buffer(&self) -> NABufferRef<Vec<u8>> { self.buffer.clone() }
}

impl Drop for NAPacket {
    fn drop(&mut self) {}
}

impl fmt::Display for NAPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut ostr = format!("[pkt for {} size {}", self.stream, self.buffer.len());
        if let Some(pts) = self.ts.pts { ostr = format!("{} pts {}", ostr, pts); }
        if let Some(dts) = self.ts.dts { ostr = format!("{} dts {}", ostr, dts); }
        if let Some(dur) = self.ts.duration { ostr = format!("{} duration {}", ostr, dur); }
        if self.keyframe { ostr = format!("{} kf", ostr); }
        ostr += "]";
        write!(f, "{}", ostr)
    }
}
