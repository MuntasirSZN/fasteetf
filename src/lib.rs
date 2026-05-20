#![no_std]

use core::mem::{align_of, size_of};
use core::slice;
use core::str;

const ETF_VERSION: u8 = 131;
const COMPRESSED_ZLIB: u8 = 80;
const BIT_BINARY_EXT: u8 = 77;
const NEW_FLOAT_EXT: u8 = 70;
const ATOM_CACHE_REF: u8 = 82;
const RECORD_EXT: u8 = 67;
const LOCAL_EXT: u8 = 121;
const NEW_PID_EXT: u8 = 88;
const NEW_PORT_EXT: u8 = 89;
const NEWER_REFERENCE_EXT: u8 = 90;

const SMALL_INTEGER_EXT: u8 = 97;
const INTEGER_EXT: u8 = 98;
const FLOAT_EXT: u8 = 99;
const ATOM_EXT: u8 = 100;
const REFERENCE_EXT: u8 = 101;
const PORT_EXT: u8 = 102;
const PID_EXT: u8 = 103;
const SMALL_TUPLE_EXT: u8 = 104;
const LARGE_TUPLE_EXT: u8 = 105;
const NIL_EXT: u8 = 106;
const STRING_EXT: u8 = 107;
const LIST_EXT: u8 = 108;
const BINARY_EXT: u8 = 109;
const SMALL_BIG_EXT: u8 = 110;
const LARGE_BIG_EXT: u8 = 111;
const NEW_FUN_EXT: u8 = 112;
const EXPORT_EXT: u8 = 113;
const NEW_REFERENCE_EXT: u8 = 114;
const FUN_EXT: u8 = 117;
const V4_PORT_EXT: u8 = 120;
const SMALL_ATOM_EXT: u8 = 115;
const MAP_EXT: u8 = 116;
const ATOM_UTF8_EXT: u8 = 118;
const SMALL_ATOM_UTF8_EXT: u8 = 119;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    InvalidVersion,
    UnexpectedEof,
    InvalidFloat,
    InvalidUtf8,
    InvalidSize,
    ArenaExhausted,
    UnsupportedTag(u8),
    TrailingData,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Term<'a> {
    SmallInteger(u8),
    Integer(i32),
    Float(f64),
    NewFloat(f64),
    Atom(&'a [u8]),
    AtomCacheRef(u8),
    Nil,
    String(&'a [u8]),
    Binary(&'a [u8]),
    BitBinary {
        bits: u8,
        data: &'a [u8],
    },
    SmallBigInt {
        sign: u8,
        digits: &'a [u8],
    },
    LargeBigInt {
        sign: u8,
        digits: &'a [u8],
    },
    Reference {
        node: &'a [u8],
        creation: u8,
        id: &'a [u8],
    },
    NewReference {
        len: u16,
        node: &'a [u8],
        creation: u8,
        id_words: &'a [u8],
    },
    NewerReference {
        len: u16,
        node: &'a [u8],
        creation: u32,
        id_words: &'a [u8],
    },
    Port {
        node: &'a [u8],
        id: u32,
        creation: u8,
    },
    NewPort {
        node: &'a [u8],
        id: u32,
        creation: u32,
    },
    V4Port {
        node: &'a [u8],
        id: u64,
        creation: u32,
    },
    Pid {
        node: &'a [u8],
        id: u32,
        serial: u32,
        creation: u8,
    },
    NewPid {
        node: &'a [u8],
        id: u32,
        serial: u32,
        creation: u32,
    },
    NewFun(NewFunView<'a>),
    Export {
        module: &'a [u8],
        function: &'a [u8],
        arity: &'a [u8],
    },
    Fun(FunView<'a>),
    Compressed {
        uncompressed_size: u32,
        compressed_data: &'a [u8],
    },
    LocalExt(&'a [u8]),
    RecordExt(&'a [u8]),
    SmallTuple(TermSeq<'a>),
    LargeTuple(TermSeq<'a>),
    List(ListView<'a>),
    Map(MapView<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewFunView<'a> {
    pub arity: u8,
    pub uniq: &'a [u8],
    pub index: u32,
    pub num_free: u32,
    pub module: &'a [u8],
    pub old_index: &'a [u8],
    pub old_uniq: &'a [u8],
    pub pid: &'a [u8],
    pub free_vars: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunView<'a> {
    pub num_free: u32,
    pub pid: &'a [u8],
    pub module: &'a [u8],
    pub index: &'a [u8],
    pub uniq: &'a [u8],
    pub free_vars: &'a [u8],
}

#[derive(Debug)]
pub struct BumpArena<'a> {
    mem: &'a mut [u8],
    pos: usize,
}

impl<'a> BumpArena<'a> {
    #[inline]
    pub fn new(mem: &'a mut [u8]) -> Self {
        Self { mem, pos: 0 }
    }

    fn alloc_raw<T>(&mut self, count: usize) -> Result<*mut T, ParseError> {
        let bytes = size_of::<T>()
            .checked_mul(count)
            .ok_or(ParseError::ArenaExhausted)?;
        let align = align_of::<T>();
        let aligned = (self.pos + (align - 1)) & !(align - 1);
        let end = aligned
            .checked_add(bytes)
            .ok_or(ParseError::ArenaExhausted)?;
        if end > self.mem.len() {
            return Err(ParseError::ArenaExhausted);
        }
        self.pos = end;
        let ptr = unsafe { self.mem.as_mut_ptr().add(aligned) as *mut T };
        Ok(ptr)
    }

    fn alloc_value<T>(&mut self, value: T) -> Result<&'a T, ParseError> {
        let ptr = self.alloc_raw::<T>(1)?;
        unsafe {
            ptr.write(value);
            Ok(&*ptr)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArenaTerm<'i, 'a> {
    SmallInteger(u8),
    Integer(i32),
    Float(f64),
    NewFloat(f64),
    Atom(&'i [u8]),
    Nil,
    String(&'i [u8]),
    Binary(&'i [u8]),
    SmallBigInt {
        sign: u8,
        digits: &'i [u8],
    },
    LargeBigInt {
        sign: u8,
        digits: &'i [u8],
    },
    SmallTuple(&'a [&'a ArenaTerm<'i, 'a>]),
    LargeTuple(&'a [&'a ArenaTerm<'i, 'a>]),
    List {
        elements: &'a [&'a ArenaTerm<'i, 'a>],
        tail: &'a ArenaTerm<'i, 'a>,
    },
    Map(&'a [(&'a ArenaTerm<'i, 'a>, &'a ArenaTerm<'i, 'a>)]),
    Other(Term<'i>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TermSeq<'a> {
    count: u32,
    payload: &'a [u8],
}

impl<'a> TermSeq<'a> {
    #[inline]
    pub fn len(&self) -> u32 {
        self.count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    pub fn iter(&self) -> TermIter<'a> {
        TermIter {
            remaining: self.count,
            cursor: Cursor {
                input: self.payload,
                pos: 0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListView<'a> {
    len: u32,
    elements_payload: &'a [u8],
    tail_payload: &'a [u8],
}

impl<'a> ListView<'a> {
    #[inline]
    pub fn len(&self) -> u32 {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn iter(&self) -> TermIter<'a> {
        TermIter {
            remaining: self.len,
            cursor: Cursor {
                input: self.elements_payload,
                pos: 0,
            },
        }
    }

    #[inline]
    pub fn tail(&self) -> Result<Term<'a>, ParseError> {
        let mut cursor = Cursor {
            input: self.tail_payload,
            pos: 0,
        };
        cursor.parse_term()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapView<'a> {
    arity: u32,
    pairs_payload: &'a [u8],
}

impl<'a> MapView<'a> {
    #[inline]
    pub fn len(&self) -> u32 {
        self.arity
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.arity == 0
    }

    #[inline]
    pub fn iter(&self) -> PairIter<'a> {
        PairIter {
            remaining: self.arity,
            cursor: Cursor {
                input: self.pairs_payload,
                pos: 0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parser<'a> {
    cursor: Cursor<'a>,
}

impl<'a> Parser<'a> {
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            cursor: Cursor {
                input: bytes,
                pos: 0,
            },
        }
    }

    #[inline]
    pub fn parse_message(mut self) -> Result<Term<'a>, ParseError> {
        let version = self.cursor.read_u8().ok_or(ParseError::Empty)?;
        if version != ETF_VERSION {
            return Err(ParseError::InvalidVersion);
        }

        let term = self.cursor.parse_term()?;
        if self.cursor.remaining_len() != 0 {
            return Err(ParseError::TrailingData);
        }
        Ok(term)
    }

    #[inline]
    pub fn parse_term(mut self) -> Result<(Term<'a>, usize), ParseError> {
        let term = self.cursor.parse_term()?;
        Ok((term, self.cursor.pos))
    }

    #[inline]
    pub fn parse_message_bump<'arena>(
        bytes: &'a [u8],
        arena: &'arena mut BumpArena<'arena>,
    ) -> Result<&'arena ArenaTerm<'a, 'arena>, ParseError> {
        let mut cursor = Cursor {
            input: bytes,
            pos: 0,
        };
        let version = cursor.read_u8().ok_or(ParseError::Empty)?;
        if version != ETF_VERSION {
            return Err(ParseError::InvalidVersion);
        }
        let term = cursor.parse_term_bump(arena as *mut BumpArena<'arena>)?;
        if cursor.remaining_len() != 0 {
            return Err(ParseError::TrailingData);
        }
        Ok(term)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Cursor<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    #[inline]
    fn remaining_len(&self) -> usize {
        self.input.len().saturating_sub(self.pos)
    }

    #[inline]
    fn read_u8(&mut self) -> Option<u8> {
        let value = *self.input.get(self.pos)?;
        self.pos += 1;
        Some(value)
    }

    #[inline]
    fn read_slice(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(len)?;
        let slice = self.input.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    #[inline]
    fn read_u16_be(&mut self) -> Option<u16> {
        let bytes = self.read_slice(2)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    #[inline]
    fn read_u32_be(&mut self) -> Option<u32> {
        let bytes = self.read_slice(4)?;
        Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    #[inline]
    fn read_u64_be(&mut self) -> Option<u64> {
        let bytes = self.read_slice(8)?;
        Some(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    #[inline]
    fn read_i32_be(&mut self) -> Option<i32> {
        let bytes = self.read_slice(4)?;
        Some(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    #[inline]
    fn take_term_payload(&mut self) -> Result<&'a [u8], ParseError> {
        let start = self.pos;
        self.skip_term()?;
        Ok(&self.input[start..self.pos])
    }

    #[inline]
    fn parse_float_ext(bytes: &[u8]) -> Result<f64, ParseError> {
        let end = find_zero_byte(bytes).unwrap_or(bytes.len());
        let parsed = str::from_utf8(&bytes[..end])
            .map_err(|_| ParseError::InvalidUtf8)?
            .trim();
        parsed.parse::<f64>().map_err(|_| ParseError::InvalidFloat)
    }

    fn parse_term(&mut self) -> Result<Term<'a>, ParseError> {
        let tag = self.read_u8().ok_or(ParseError::UnexpectedEof)?;

        match tag {
            COMPRESSED_ZLIB => {
                let uncompressed_size = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let payload = self
                    .read_slice(self.remaining_len())
                    .ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Compressed {
                    uncompressed_size,
                    compressed_data: payload,
                })
            }
            BIT_BINARY_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let bits = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                let data = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::BitBinary { bits, data })
            }
            SMALL_INTEGER_EXT => {
                let value = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::SmallInteger(value))
            }
            INTEGER_EXT => {
                let value = self.read_i32_be().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Integer(value))
            }
            FLOAT_EXT => {
                let bytes = self.read_slice(31).ok_or(ParseError::UnexpectedEof)?;
                let value = Self::parse_float_ext(bytes)?;
                Ok(Term::Float(value))
            }
            NEW_FLOAT_EXT => {
                let bytes = self.read_slice(8).ok_or(ParseError::UnexpectedEof)?;
                let value = f64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Ok(Term::NewFloat(value))
            }
            ATOM_CACHE_REF => {
                let idx = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::AtomCacheRef(idx))
            }
            ATOM_EXT | ATOM_UTF8_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let atom = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Atom(atom))
            }
            SMALL_ATOM_EXT | SMALL_ATOM_UTF8_EXT => {
                let len = self.read_u8().ok_or(ParseError::UnexpectedEof)? as usize;
                let atom = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Atom(atom))
            }
            NIL_EXT => Ok(Term::Nil),
            STRING_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let bytes = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::String(bytes))
            }
            BINARY_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let bytes = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Binary(bytes))
            }
            SMALL_BIG_EXT => {
                let len = self.read_u8().ok_or(ParseError::UnexpectedEof)? as usize;
                let sign = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                let digits = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::SmallBigInt { sign, digits })
            }
            LARGE_BIG_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let sign = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                let digits = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::LargeBigInt { sign, digits })
            }
            REFERENCE_EXT => {
                let node = self.take_term_payload()?;
                let id = self.read_slice(4).ok_or(ParseError::UnexpectedEof)?;
                let creation = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Reference { node, creation, id })
            }
            NEW_REFERENCE_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)?;
                let node = self.take_term_payload()?;
                let creation = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                let id_words_len = (len as usize)
                    .checked_mul(4)
                    .ok_or(ParseError::InvalidSize)?;
                let id_words = self
                    .read_slice(id_words_len)
                    .ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::NewReference {
                    len,
                    node,
                    creation,
                    id_words,
                })
            }
            NEWER_REFERENCE_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)?;
                let node = self.take_term_payload()?;
                let creation = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let id_words_len = (len as usize)
                    .checked_mul(4)
                    .ok_or(ParseError::InvalidSize)?;
                let id_words = self
                    .read_slice(id_words_len)
                    .ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::NewerReference {
                    len,
                    node,
                    creation,
                    id_words,
                })
            }
            PORT_EXT => {
                let node = self.take_term_payload()?;
                let id = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let creation = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Port { node, id, creation })
            }
            NEW_PORT_EXT => {
                let node = self.take_term_payload()?;
                let id = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let creation = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::NewPort { node, id, creation })
            }
            V4_PORT_EXT => {
                let node = self.take_term_payload()?;
                let id = self.read_u64_be().ok_or(ParseError::UnexpectedEof)?;
                let creation = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::V4Port { node, id, creation })
            }
            PID_EXT => {
                let node = self.take_term_payload()?;
                let id = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let serial = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let creation = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Pid {
                    node,
                    id,
                    serial,
                    creation,
                })
            }
            NEW_PID_EXT => {
                let node = self.take_term_payload()?;
                let id = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let serial = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let creation = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::NewPid {
                    node,
                    id,
                    serial,
                    creation,
                })
            }
            NEW_FUN_EXT => {
                let size = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                if size < 4 {
                    return Err(ParseError::InvalidSize);
                }
                let payload_start = self.pos;
                let arity = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                let uniq = self.read_slice(16).ok_or(ParseError::UnexpectedEof)?;
                let index = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let num_free = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let module = self.take_term_payload()?;
                let old_index = self.take_term_payload()?;
                let old_uniq = self.take_term_payload()?;
                let pid = self.take_term_payload()?;
                let free_vars = self.take_compound_payload(num_free)?;
                let consumed = self.pos - payload_start;
                if consumed != (size - 4) {
                    return Err(ParseError::InvalidSize);
                }
                Ok(Term::NewFun(NewFunView {
                    arity,
                    uniq,
                    index,
                    num_free,
                    module,
                    old_index,
                    old_uniq,
                    pid,
                    free_vars,
                }))
            }
            EXPORT_EXT => {
                let module = self.take_term_payload()?;
                let function = self.take_term_payload()?;
                let arity = self.take_term_payload()?;
                Ok(Term::Export {
                    module,
                    function,
                    arity,
                })
            }
            FUN_EXT => {
                let num_free = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let pid = self.take_term_payload()?;
                let module = self.take_term_payload()?;
                let index = self.take_term_payload()?;
                let uniq = self.take_term_payload()?;
                let free_vars = self.take_compound_payload(num_free)?;
                Ok(Term::Fun(FunView {
                    num_free,
                    pid,
                    module,
                    index,
                    uniq,
                    free_vars,
                }))
            }
            LOCAL_EXT => {
                let payload = self
                    .read_slice(self.remaining_len())
                    .ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::LocalExt(payload))
            }
            RECORD_EXT => {
                let payload = self
                    .read_slice(self.remaining_len())
                    .ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::RecordExt(payload))
            }
            SMALL_TUPLE_EXT => {
                let count = self.read_u8().ok_or(ParseError::UnexpectedEof)? as u32;
                let payload = self.take_compound_payload(count)?;
                Ok(Term::SmallTuple(TermSeq { count, payload }))
            }
            LARGE_TUPLE_EXT => {
                let count = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let payload = self.take_compound_payload(count)?;
                Ok(Term::LargeTuple(TermSeq { count, payload }))
            }
            LIST_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let elements_payload = self.take_compound_payload(len)?;
                let tail_start = self.pos;
                self.skip_term()?;
                let tail_payload = &self.input[tail_start..self.pos];
                Ok(Term::List(ListView {
                    len,
                    elements_payload,
                    tail_payload,
                }))
            }
            MAP_EXT => {
                let arity = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                let pairs_payload = self.take_compound_payload(arity.saturating_mul(2))?;
                Ok(Term::Map(MapView {
                    arity,
                    pairs_payload,
                }))
            }
            _ => Err(ParseError::UnsupportedTag(tag)),
        }
    }

    fn take_compound_payload(&mut self, terms: u32) -> Result<&'a [u8], ParseError> {
        let start = self.pos;
        for _ in 0..terms {
            self.skip_term()?;
        }
        Ok(&self.input[start..self.pos])
    }

    fn parse_term_bump<'arena>(
        &mut self,
        arena: *mut BumpArena<'arena>,
    ) -> Result<&'arena ArenaTerm<'a, 'arena>, ParseError> {
        let tag = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
        let term = match tag {
            SMALL_INTEGER_EXT => {
                ArenaTerm::SmallInteger(self.read_u8().ok_or(ParseError::UnexpectedEof)?)
            }
            INTEGER_EXT => ArenaTerm::Integer(self.read_i32_be().ok_or(ParseError::UnexpectedEof)?),
            FLOAT_EXT => {
                let bytes = self.read_slice(31).ok_or(ParseError::UnexpectedEof)?;
                ArenaTerm::Float(Self::parse_float_ext(bytes)?)
            }
            NEW_FLOAT_EXT => {
                let bytes = self.read_slice(8).ok_or(ParseError::UnexpectedEof)?;
                ArenaTerm::NewFloat(f64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]))
            }
            ATOM_EXT | ATOM_UTF8_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                ArenaTerm::Atom(self.read_slice(len).ok_or(ParseError::UnexpectedEof)?)
            }
            SMALL_ATOM_EXT | SMALL_ATOM_UTF8_EXT => {
                let len = self.read_u8().ok_or(ParseError::UnexpectedEof)? as usize;
                ArenaTerm::Atom(self.read_slice(len).ok_or(ParseError::UnexpectedEof)?)
            }
            NIL_EXT => ArenaTerm::Nil,
            STRING_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                ArenaTerm::String(self.read_slice(len).ok_or(ParseError::UnexpectedEof)?)
            }
            BINARY_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                ArenaTerm::Binary(self.read_slice(len).ok_or(ParseError::UnexpectedEof)?)
            }
            SMALL_BIG_EXT => {
                let len = self.read_u8().ok_or(ParseError::UnexpectedEof)? as usize;
                let sign = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                let digits = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                ArenaTerm::SmallBigInt { sign, digits }
            }
            LARGE_BIG_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let sign = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                let digits = self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
                ArenaTerm::LargeBigInt { sign, digits }
            }
            SMALL_TUPLE_EXT => {
                let count = self.read_u8().ok_or(ParseError::UnexpectedEof)? as usize;
                let ptr =
                    unsafe { (&mut *arena).alloc_raw::<&'arena ArenaTerm<'a, 'arena>>(count)? };
                for i in 0..count {
                    let child = self.parse_term_bump(arena)?;
                    unsafe { ptr.add(i).write(child) };
                }
                let elems = unsafe { slice::from_raw_parts(ptr, count) };
                ArenaTerm::SmallTuple(elems)
            }
            LARGE_TUPLE_EXT => {
                let count = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let ptr =
                    unsafe { (&mut *arena).alloc_raw::<&'arena ArenaTerm<'a, 'arena>>(count)? };
                for i in 0..count {
                    let child = self.parse_term_bump(arena)?;
                    unsafe { ptr.add(i).write(child) };
                }
                let elems = unsafe { slice::from_raw_parts(ptr, count) };
                ArenaTerm::LargeTuple(elems)
            }
            LIST_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let ptr = unsafe { (&mut *arena).alloc_raw::<&'arena ArenaTerm<'a, 'arena>>(len)? };
                for i in 0..len {
                    let child = self.parse_term_bump(arena)?;
                    unsafe { ptr.add(i).write(child) };
                }
                let elements = unsafe { slice::from_raw_parts(ptr, len) };
                let tail = self.parse_term_bump(arena)?;
                ArenaTerm::List { elements, tail }
            }
            MAP_EXT => {
                let arity = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let ptr = unsafe {
                    (&mut *arena)
                        .alloc_raw::<(&'arena ArenaTerm<'a, 'arena>, &'arena ArenaTerm<'a, 'arena>)>(
                            arity,
                        )?
                };
                for i in 0..arity {
                    let key = self.parse_term_bump(arena)?;
                    let value = self.parse_term_bump(arena)?;
                    unsafe { ptr.add(i).write((key, value)) };
                }
                let pairs = unsafe { slice::from_raw_parts(ptr, arity) };
                ArenaTerm::Map(pairs)
            }
            _ => ArenaTerm::Other(self.parse_term_from_known_tag(tag)?),
        };
        unsafe { (&mut *arena).alloc_value(term) }
    }

    fn parse_term_from_known_tag(&mut self, tag: u8) -> Result<Term<'a>, ParseError> {
        self.pos = self.pos.saturating_sub(1);
        let reparsed = self.parse_term()?;
        match reparsed {
            Term::SmallTuple(_) | Term::LargeTuple(_) | Term::List(_) | Term::Map(_) => {
                if tag == SMALL_TUPLE_EXT
                    || tag == LARGE_TUPLE_EXT
                    || tag == LIST_EXT
                    || tag == MAP_EXT
                {
                    return Err(ParseError::UnsupportedTag(tag));
                }
                Ok(reparsed)
            }
            _ => Ok(reparsed),
        }
    }

    fn skip_term(&mut self) -> Result<(), ParseError> {
        let tag = self.read_u8().ok_or(ParseError::UnexpectedEof)?;

        match tag {
            COMPRESSED_ZLIB => {
                self.read_slice(4).ok_or(ParseError::UnexpectedEof)?;
                let rem = self.remaining_len();
                self.read_slice(rem).ok_or(ParseError::UnexpectedEof)?;
            }
            BIT_BINARY_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                self.read_slice(1 + len).ok_or(ParseError::UnexpectedEof)?;
            }
            SMALL_INTEGER_EXT => {
                self.read_slice(1).ok_or(ParseError::UnexpectedEof)?;
            }
            INTEGER_EXT => {
                self.read_slice(4).ok_or(ParseError::UnexpectedEof)?;
            }
            FLOAT_EXT => {
                self.read_slice(31).ok_or(ParseError::UnexpectedEof)?;
            }
            NEW_FLOAT_EXT => {
                self.read_slice(8).ok_or(ParseError::UnexpectedEof)?;
            }
            ATOM_CACHE_REF => {
                self.read_slice(1).ok_or(ParseError::UnexpectedEof)?;
            }
            ATOM_EXT | ATOM_UTF8_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
            }
            SMALL_ATOM_EXT | SMALL_ATOM_UTF8_EXT => {
                let len = self.read_u8().ok_or(ParseError::UnexpectedEof)? as usize;
                self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
            }
            NIL_EXT => {}
            STRING_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
            }
            BINARY_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                self.read_slice(len).ok_or(ParseError::UnexpectedEof)?;
            }
            SMALL_BIG_EXT => {
                let len = self.read_u8().ok_or(ParseError::UnexpectedEof)? as usize;
                self.read_slice(1 + len).ok_or(ParseError::UnexpectedEof)?;
            }
            LARGE_BIG_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                self.read_slice(1 + len).ok_or(ParseError::UnexpectedEof)?;
            }
            REFERENCE_EXT => {
                self.skip_term()?;
                self.read_slice(4 + 1).ok_or(ParseError::UnexpectedEof)?;
            }
            NEW_REFERENCE_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                self.skip_term()?;
                self.read_slice(1 + (len * 4))
                    .ok_or(ParseError::UnexpectedEof)?;
            }
            NEWER_REFERENCE_EXT => {
                let len = self.read_u16_be().ok_or(ParseError::UnexpectedEof)? as usize;
                self.skip_term()?;
                self.read_slice(4 + (len * 4))
                    .ok_or(ParseError::UnexpectedEof)?;
            }
            PORT_EXT => {
                self.skip_term()?;
                self.read_slice(4 + 1).ok_or(ParseError::UnexpectedEof)?;
            }
            NEW_PORT_EXT => {
                self.skip_term()?;
                self.read_slice(4 + 4).ok_or(ParseError::UnexpectedEof)?;
            }
            V4_PORT_EXT => {
                self.skip_term()?;
                self.read_slice(8 + 4).ok_or(ParseError::UnexpectedEof)?;
            }
            PID_EXT => {
                self.skip_term()?;
                self.read_slice(4 + 4 + 1)
                    .ok_or(ParseError::UnexpectedEof)?;
            }
            NEW_PID_EXT => {
                self.skip_term()?;
                self.read_slice(4 + 4 + 4)
                    .ok_or(ParseError::UnexpectedEof)?;
            }
            NEW_FUN_EXT => {
                let size = self.read_u32_be().ok_or(ParseError::UnexpectedEof)? as usize;
                let payload = size.checked_sub(4).ok_or(ParseError::InvalidSize)?;
                self.read_slice(payload).ok_or(ParseError::UnexpectedEof)?;
            }
            EXPORT_EXT => {
                self.skip_term()?;
                self.skip_term()?;
                self.skip_term()?;
            }
            FUN_EXT => {
                let num_free = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                self.skip_term()?;
                self.skip_term()?;
                self.skip_term()?;
                self.skip_term()?;
                for _ in 0..num_free {
                    self.skip_term()?;
                }
            }
            LOCAL_EXT | RECORD_EXT => {
                let rem = self.remaining_len();
                self.read_slice(rem).ok_or(ParseError::UnexpectedEof)?;
            }
            SMALL_TUPLE_EXT => {
                let count = self.read_u8().ok_or(ParseError::UnexpectedEof)? as u32;
                for _ in 0..count {
                    self.skip_term()?;
                }
            }
            LARGE_TUPLE_EXT => {
                let count = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                for _ in 0..count {
                    self.skip_term()?;
                }
            }
            LIST_EXT => {
                let len = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                for _ in 0..len {
                    self.skip_term()?;
                }
                self.skip_term()?;
            }
            MAP_EXT => {
                let arity = self.read_u32_be().ok_or(ParseError::UnexpectedEof)?;
                for _ in 0..arity {
                    self.skip_term()?;
                    self.skip_term()?;
                }
            }
            _ => return Err(ParseError::UnsupportedTag(tag)),
        }

        Ok(())
    }
}

#[inline]
fn find_zero_byte(bytes: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        return find_zero_byte_sse2(bytes);
    }
    #[allow(unreachable_code)]
    bytes.iter().position(|&b| b == 0)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn find_zero_byte_sse2(bytes: &[u8]) -> Option<usize> {
    use core::arch::x86_64::*;
    let mut i = 0usize;
    while i + 16 <= bytes.len() {
        let chunk = unsafe { _mm_loadu_si128(bytes.as_ptr().add(i) as *const __m128i) };
        let zeros = _mm_setzero_si128();
        let cmp = _mm_cmpeq_epi8(chunk, zeros);
        let mask = _mm_movemask_epi8(cmp) as u32;
        if mask != 0 {
            return Some(i + mask.trailing_zeros() as usize);
        }
        i += 16;
    }
    for (idx, b) in bytes[i..].iter().enumerate() {
        if *b == 0 {
            return Some(i + idx);
        }
    }
    None
}

pub struct TermIter<'a> {
    remaining: u32,
    cursor: Cursor<'a>,
}

impl<'a> Iterator for TermIter<'a> {
    type Item = Result<Term<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;
        Some(self.cursor.parse_term())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.remaining as usize;
        (len, Some(len))
    }
}

pub struct PairIter<'a> {
    remaining: u32,
    cursor: Cursor<'a>,
}

impl<'a> Iterator for PairIter<'a> {
    type Item = Result<(Term<'a>, Term<'a>), ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        let key = match self.cursor.parse_term() {
            Ok(value) => value,
            Err(err) => return Some(Err(err)),
        };

        let value = match self.cursor.parse_term() {
            Ok(value) => value,
            Err(err) => return Some(Err(err)),
        };

        Some(Ok((key, value)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.remaining as usize;
        (len, Some(len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_small_integer_message() {
        let input = [ETF_VERSION, SMALL_INTEGER_EXT, 42];
        let term = Parser::new(&input).parse_message().unwrap();
        assert_eq!(term, Term::SmallInteger(42));
    }

    #[test]
    fn parses_binary_zero_copy() {
        let input = [ETF_VERSION, BINARY_EXT, 0, 0, 0, 3, 1, 2, 3];
        let term = Parser::new(&input).parse_message().unwrap();

        match term {
            Term::Binary(bytes) => assert_eq!(bytes, &[1, 2, 3]),
            _ => panic!("unexpected term"),
        }
    }

    #[test]
    fn parses_small_tuple_and_iterates_elements() {
        let input = [
            ETF_VERSION,
            SMALL_TUPLE_EXT,
            2,
            SMALL_INTEGER_EXT,
            1,
            SMALL_INTEGER_EXT,
            2,
        ];

        let term = Parser::new(&input).parse_message().unwrap();
        let seq = match term {
            Term::SmallTuple(seq) => seq,
            _ => panic!("unexpected term"),
        };

        let values: [u8; 2] = {
            let mut out = [0_u8; 2];
            for (i, item) in seq.iter().enumerate() {
                out[i] = match item.unwrap() {
                    Term::SmallInteger(v) => v,
                    _ => panic!("unexpected tuple element"),
                };
            }
            out
        };

        assert_eq!(values, [1, 2]);
    }

    #[test]
    fn parses_list_with_tail() {
        let input = [
            ETF_VERSION,
            LIST_EXT,
            0,
            0,
            0,
            2,
            SMALL_INTEGER_EXT,
            7,
            SMALL_INTEGER_EXT,
            8,
            NIL_EXT,
        ];

        let term = Parser::new(&input).parse_message().unwrap();
        let list = match term {
            Term::List(list) => list,
            _ => panic!("unexpected term"),
        };

        let mut it = list.iter();
        assert_eq!(it.next().unwrap().unwrap(), Term::SmallInteger(7));
        assert_eq!(it.next().unwrap().unwrap(), Term::SmallInteger(8));
        assert!(it.next().is_none());
        assert_eq!(list.tail().unwrap(), Term::Nil);
    }

    #[test]
    fn parses_map_pairs() {
        let input = [
            ETF_VERSION,
            MAP_EXT,
            0,
            0,
            0,
            1,
            SMALL_INTEGER_EXT,
            1,
            SMALL_INTEGER_EXT,
            9,
        ];

        let term = Parser::new(&input).parse_message().unwrap();
        let map = match term {
            Term::Map(map) => map,
            _ => panic!("unexpected term"),
        };

        let pair = map.iter().next().unwrap().unwrap();
        assert_eq!(pair.0, Term::SmallInteger(1));
        assert_eq!(pair.1, Term::SmallInteger(9));
    }

    #[test]
    fn errors_on_invalid_version() {
        let input = [0, SMALL_INTEGER_EXT, 1];
        let err = Parser::new(&input).parse_message().unwrap_err();
        assert_eq!(err, ParseError::InvalidVersion);
    }

    #[test]
    fn errors_on_trailing_data() {
        let input = [ETF_VERSION, SMALL_INTEGER_EXT, 1, SMALL_INTEGER_EXT, 2];
        let err = Parser::new(&input).parse_message().unwrap_err();
        assert_eq!(err, ParseError::TrailingData);
    }

    #[test]
    fn parses_legacy_float_ext() {
        let mut input = [0_u8; 1 + 1 + 31];
        input[0] = ETF_VERSION;
        input[1] = FLOAT_EXT;
        let ascii = b"1.25000000000000000000e+00";
        input[2..2 + ascii.len()].copy_from_slice(ascii);
        input[2 + ascii.len()] = 0;

        let term = Parser::new(&input).parse_message().unwrap();
        assert_eq!(term, Term::Float(1.25));
    }

    #[test]
    fn parses_reference_ext() {
        let input = [
            ETF_VERSION,
            REFERENCE_EXT,
            SMALL_ATOM_UTF8_EXT,
            4,
            b'n',
            b'o',
            b'd',
            b'e',
            0,
            0,
            0,
            7,
            1,
        ];

        let term = Parser::new(&input).parse_message().unwrap();
        match term {
            Term::Reference { node, creation, id } => {
                assert_eq!(creation, 1);
                assert_eq!(id, &[0, 0, 0, 7]);
                let node_term = Parser::new(node).parse_term().unwrap().0;
                assert_eq!(node_term, Term::Atom(b"node"));
            }
            _ => panic!("unexpected term"),
        }
    }

    #[test]
    fn parses_export_ext() {
        let input = [
            ETF_VERSION,
            EXPORT_EXT,
            SMALL_ATOM_UTF8_EXT,
            1,
            b'm',
            SMALL_ATOM_UTF8_EXT,
            1,
            b'f',
            SMALL_INTEGER_EXT,
            2,
        ];
        let term = Parser::new(&input).parse_message().unwrap();
        match term {
            Term::Export {
                module,
                function,
                arity,
            } => {
                assert_eq!(
                    Parser::new(module).parse_term().unwrap().0,
                    Term::Atom(b"m")
                );
                assert_eq!(
                    Parser::new(function).parse_term().unwrap().0,
                    Term::Atom(b"f")
                );
                assert_eq!(
                    Parser::new(arity).parse_term().unwrap().0,
                    Term::SmallInteger(2)
                );
            }
            _ => panic!("unexpected term"),
        }
    }

    #[test]
    fn parses_message_with_bump_recursive_descent() {
        let input = [
            ETF_VERSION,
            SMALL_TUPLE_EXT,
            2,
            LIST_EXT,
            0,
            0,
            0,
            2,
            SMALL_INTEGER_EXT,
            7,
            SMALL_INTEGER_EXT,
            8,
            NIL_EXT,
            MAP_EXT,
            0,
            0,
            0,
            1,
            SMALL_INTEGER_EXT,
            1,
            SMALL_INTEGER_EXT,
            9,
        ];
        let mut mem = [0_u8; 2048];
        let mut arena = BumpArena::new(&mut mem);
        let term = Parser::parse_message_bump(&input, &mut arena).unwrap();

        let tuple = match term {
            ArenaTerm::SmallTuple(items) => items,
            _ => panic!("unexpected top-level term"),
        };
        assert_eq!(tuple.len(), 2);
        match tuple[0] {
            ArenaTerm::List { elements, tail } => {
                assert_eq!(elements.len(), 2);
                assert_eq!(*elements[0], ArenaTerm::SmallInteger(7));
                assert_eq!(*elements[1], ArenaTerm::SmallInteger(8));
                assert_eq!(**tail, ArenaTerm::Nil);
            }
            _ => panic!("expected list"),
        }
        match tuple[1] {
            ArenaTerm::Map(pairs) => {
                assert_eq!(pairs.len(), 1);
                assert_eq!(*pairs[0].0, ArenaTerm::SmallInteger(1));
                assert_eq!(*pairs[0].1, ArenaTerm::SmallInteger(9));
            }
            _ => panic!("expected map"),
        }
    }
}
