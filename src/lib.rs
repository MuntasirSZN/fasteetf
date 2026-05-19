#![no_std]

const ETF_VERSION: u8 = 131;

const SMALL_INTEGER_EXT: u8 = 97;
const INTEGER_EXT: u8 = 98;
const NEW_FLOAT_EXT: u8 = 70;
const ATOM_EXT: u8 = 100;
const SMALL_TUPLE_EXT: u8 = 104;
const LARGE_TUPLE_EXT: u8 = 105;
const NIL_EXT: u8 = 106;
const STRING_EXT: u8 = 107;
const LIST_EXT: u8 = 108;
const BINARY_EXT: u8 = 109;
const SMALL_BIG_EXT: u8 = 110;
const LARGE_BIG_EXT: u8 = 111;
const SMALL_ATOM_EXT: u8 = 115;
const MAP_EXT: u8 = 116;
const ATOM_UTF8_EXT: u8 = 118;
const SMALL_ATOM_UTF8_EXT: u8 = 119;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    InvalidVersion,
    UnexpectedEof,
    UnsupportedTag(u8),
    TrailingData,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Term<'a> {
    SmallInteger(u8),
    Integer(i32),
    NewFloat(f64),
    Atom(&'a [u8]),
    Nil,
    String(&'a [u8]),
    Binary(&'a [u8]),
    SmallBigInt { sign: u8, digits: &'a [u8] },
    LargeBigInt { sign: u8, digits: &'a [u8] },
    SmallTuple(TermSeq<'a>),
    LargeTuple(TermSeq<'a>),
    List(ListView<'a>),
    Map(MapView<'a>),
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
    fn read_i32_be(&mut self) -> Option<i32> {
        let bytes = self.read_slice(4)?;
        Some(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn parse_term(&mut self) -> Result<Term<'a>, ParseError> {
        let tag = self.read_u8().ok_or(ParseError::UnexpectedEof)?;

        match tag {
            SMALL_INTEGER_EXT => {
                let value = self.read_u8().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::SmallInteger(value))
            }
            INTEGER_EXT => {
                let value = self.read_i32_be().ok_or(ParseError::UnexpectedEof)?;
                Ok(Term::Integer(value))
            }
            NEW_FLOAT_EXT => {
                let bytes = self.read_slice(8).ok_or(ParseError::UnexpectedEof)?;
                let value = f64::from_be_bytes([
                    bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                ]);
                Ok(Term::NewFloat(value))
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

    fn skip_term(&mut self) -> Result<(), ParseError> {
        let tag = self.read_u8().ok_or(ParseError::UnexpectedEof)?;

        match tag {
            SMALL_INTEGER_EXT => {
                self.read_slice(1).ok_or(ParseError::UnexpectedEof)?;
            }
            INTEGER_EXT => {
                self.read_slice(4).ok_or(ParseError::UnexpectedEof)?;
            }
            NEW_FLOAT_EXT => {
                self.read_slice(8).ok_or(ParseError::UnexpectedEof)?;
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
}
