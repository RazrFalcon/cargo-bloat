use std::{str, mem};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
}

pub trait RawNumber: Sized {
    fn parse(s: &mut Stream) -> Self;
}

macro_rules! cast_number {
    ($num:ty, $data:expr) => {{
        assert_eq!($data.len(), std::mem::size_of::<$num>());
        let mut num: $num = 0;
        unsafe {
            core::ptr::copy_nonoverlapping(
                $data.as_ptr(),
                &mut num as *mut $num as *mut u8,
                std::mem::size_of::<$num>(),
            );
        }
        num
    }};
}

impl RawNumber for u8 {
    #[inline]
    fn parse(s: &mut Stream) -> Self {
        s.data[s.offset]
    }
}

impl RawNumber for i8 {
    #[inline]
    fn parse(s: &mut Stream) -> Self {
        s.data[s.offset] as i8
    }
}

impl RawNumber for u16 {
    #[inline]
    fn parse(s: &mut Stream) -> Self {
        let start = s.offset;
        let end = s.offset + mem::size_of::<Self>();
        let num = cast_number!(Self, s.data[start..end]);
        match s.byte_order {
            ByteOrder::LittleEndian => num,
            ByteOrder::BigEndian => num.to_be(),
        }
    }
}

impl RawNumber for i16 {
    #[inline]
    fn parse(s: &mut Stream) -> Self {
        s.read::<u16>() as i16
    }
}

impl RawNumber for u32 {
    #[inline]
    fn parse(s: &mut Stream) -> Self {
        let start = s.offset;
        let end = s.offset + mem::size_of::<Self>();
        let num = cast_number!(Self, s.data[start..end]);
        match s.byte_order {
            ByteOrder::LittleEndian => num,
            ByteOrder::BigEndian => num.to_be(),
        }
    }
}

impl RawNumber for u64 {
    #[inline]
    fn parse(s: &mut Stream) -> Self {
        let start = s.offset;
        let end = s.offset + mem::size_of::<Self>();
        let num = cast_number!(Self, s.data[start..end]);
        match s.byte_order {
            ByteOrder::LittleEndian => num,
            ByteOrder::BigEndian => num.to_be(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Stream<'a> {
    data: &'a [u8],
    offset: usize,
    byte_order: ByteOrder,
}

impl<'a> Stream<'a> {
    #[inline]
    pub fn new(data: &'a [u8], byte_order: ByteOrder) -> Self {
        Stream {
            data,
            offset: 0,
            byte_order,
        }
    }

    #[inline]
    pub fn new_at(data: &'a [u8], offset: usize, byte_order: ByteOrder) -> Self {
        Stream {
            data,
            offset,
            byte_order,
        }
    }

    #[inline]
    pub fn byte_order(&self) -> ByteOrder {
        self.byte_order
    }

    #[inline]
    pub fn at_end(&self) -> bool {
        self.offset == self.data.len()
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    #[inline]
    pub fn skip<T: RawNumber>(&mut self) {
        self.offset += mem::size_of::<T>();
    }

    #[inline]
    pub fn skip_len(&mut self, len: usize) {
        self.offset += len;
    }

    #[inline]
    pub fn read<T: RawNumber>(&mut self) -> T {
        let start = self.offset;
        let v = T::parse(self);
        self.offset = start + mem::size_of::<T>();
        v
    }

    #[inline]
    pub fn read_bytes(&mut self, len: usize) -> &'a [u8] {
        let offset = self.offset;
        self.offset += len;
        &self.data[offset..(offset + len)]
    }
}

pub fn parse_null_string(data: &[u8], start: usize) -> Option<&str> {
    match data[start..].iter().position(|c| *c == b'\0') {
        Some(i) if i != 0 => str::from_utf8(&data[start..start+i]).ok(),
        _ => None,
    }
}
