use crate::encoder::{Sink, encode_atom, encode_int, encode_small_big};
use crate::error::EtfError;
use crate::tags::*;

/// A fixed-capacity [`Sink`] that records every byte written.
struct Probe {
    buf: [u8; 400],
    len: usize,
}

impl Sink for Probe {
    fn write_u8(&mut self, v: u8) -> Result<(), EtfError> {
        self.buf[self.len] = v;
        self.len += 1;
        Ok(())
    }

    fn write_u16(&mut self, v: u16) -> Result<(), EtfError> {
        self.buf[self.len..self.len + 2].copy_from_slice(&v.to_be_bytes());
        self.len += 2;
        Ok(())
    }

    fn write_u32(&mut self, v: u32) -> Result<(), EtfError> {
        self.buf[self.len..self.len + 4].copy_from_slice(&v.to_be_bytes());
        self.len += 4;
        Ok(())
    }

    fn write_f64(&mut self, v: f64) -> Result<(), EtfError> {
        self.buf[self.len..self.len + 8].copy_from_slice(&v.to_be_bytes());
        self.len += 8;
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), EtfError> {
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

#[kani::proof]
fn encode_int_writes_compact_form() {
    let v: i32 = kani::any();
    let mut p = Probe {
        buf: [0; 400],
        len: 0,
    };
    assert!(encode_int(&mut p, v).is_ok());
    if (0..=255).contains(&v) {
        assert_eq!(p.buf[0], SMALL_INTEGER_EXT);
        assert_eq!(p.buf[1], v as u8);
        assert_eq!(p.len, 2);
    } else {
        assert_eq!(p.buf[0], INTEGER_EXT);
        assert_eq!(p.len, 5);
    }
}

#[kani::proof]
#[kani::unwind(33)]
fn encode_atom_small_form() {
    let bytes: [u8; 32] = kani::any();
    let len: u8 = kani::any();
    kani::assume(len <= 32);
    let b = &bytes[..len as usize];
    let mut p = Probe {
        buf: [0; 400],
        len: 0,
    };
    assert!(encode_atom(&mut p, b).is_ok());
    assert_eq!(p.buf[0], SMALL_ATOM_UTF8_EXT);
    assert_eq!(p.buf[1], len);
    assert_eq!(p.len, 2 + len as usize);
    for i in 0..len as usize {
        assert_eq!(p.buf[2 + i], b[i]);
    }
}

#[kani::proof]
fn encode_atom_large_form() {
    let bytes: [u8; 300] = kani::any();
    let len: u16 = kani::any();
    kani::assume(len >= 256 && len <= 300);
    let b = &bytes[..len as usize];
    let mut p = Probe {
        buf: [0; 400],
        len: 0,
    };
    assert!(encode_atom(&mut p, b).is_ok());
    assert_eq!(p.buf[0], ATOM_UTF8_EXT);
    assert_eq!(u16::from_be_bytes([p.buf[1], p.buf[2]]), len);
    assert_eq!(p.len, 3 + len as usize);
    assert_eq!(p.buf[3], b[0]);
    assert_eq!(p.buf[3 + len as usize - 1], b[len as usize - 1]);
}

#[kani::proof]
#[kani::unwind(33)]
fn encode_small_big_small_form() {
    let digits: [u8; 32] = kani::any();
    let len: u8 = kani::any();
    kani::assume(len <= 32);
    let d = &digits[..len as usize];
    let mut p = Probe {
        buf: [0; 400],
        len: 0,
    };
    assert!(encode_small_big(&mut p, 0, d).is_ok());
    assert_eq!(p.buf[0], SMALL_BIG_EXT);
    assert_eq!(p.buf[1], len);
    assert_eq!(p.buf[2], 0);
    assert_eq!(p.len, 3 + len as usize);
    for i in 0..len as usize {
        assert_eq!(p.buf[3 + i], d[i]);
    }
}

#[kani::proof]
fn encode_small_big_upgrades_on_overflow() {
    let digits: [u8; 300] = kani::any();
    let len: u16 = kani::any();
    kani::assume(len > 255 && len <= 300);
    let d = &digits[..len as usize];
    let mut p = Probe {
        buf: [0; 400],
        len: 0,
    };
    assert!(encode_small_big(&mut p, 0, d).is_ok());
    assert_eq!(p.buf[0], LARGE_BIG_EXT);
    assert_eq!(
        u32::from_be_bytes([p.buf[1], p.buf[2], p.buf[3], p.buf[4]]),
        len as u32
    );
    assert_eq!(p.buf[5], 0);
    assert_eq!(p.len, 6 + len as usize);
    assert_eq!(p.buf[6], d[0]);
}
