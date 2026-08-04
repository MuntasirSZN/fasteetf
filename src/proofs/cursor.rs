use crate::cursor::Cursor;
use crate::error::{EtfError, Needed};

#[kani::proof]
fn cursor_read_u8() {
    let bytes: [u8; 4] = kani::any();
    let len: u8 = kani::any();
    kani::assume(len <= 4);
    let data = &bytes[..len as usize];
    let mut c = Cursor::new(data);
    match c.read_u8() {
        Ok(b) => {
            assert_eq!(b, bytes[0]);
            assert_eq!(c.data.len(), len as usize - 1);
            assert_eq!(c.consumed(), 1);
        }
        Err(e) => {
            assert_eq!(len, 0);
            assert_eq!(e, EtfError::UnexpectedEof);
        }
    }
}

#[kani::proof]
fn cursor_read_u16() {
    let bytes: [u8; 4] = kani::any();
    let len: u8 = kani::any();
    kani::assume(len <= 4);
    let data = &bytes[..len as usize];
    let mut c = Cursor::new(data);
    match c.read_u16() {
        Ok(v) => {
            assert_eq!(v, u16::from_be_bytes([bytes[0], bytes[1]]));
            assert_eq!(c.data.len(), len as usize - 2);
        }
        Err(e) => {
            assert!(len < 2);
            assert_eq!(e, EtfError::UnexpectedEof);
        }
    }
}

#[kani::proof]
fn cursor_read_u32() {
    let bytes: [u8; 8] = kani::any();
    let len: u8 = kani::any();
    kani::assume(len <= 8);
    let data = &bytes[..len as usize];
    let mut c = Cursor::new(data);
    match c.read_u32() {
        Ok(v) => {
            assert_eq!(
                v,
                u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            );
            assert_eq!(c.data.len(), len as usize - 4);
        }
        Err(e) => {
            assert!(len < 4);
            assert_eq!(e, EtfError::UnexpectedEof);
        }
    }
}

#[kani::proof]
fn cursor_read_f64() {
    let bytes: [u8; 8] = kani::any();
    let len: u8 = kani::any();
    kani::assume(len <= 8);
    let data = &bytes[..len as usize];
    let mut c = Cursor::new(data);
    match c.read_f64() {
        Ok(v) => {
            assert_eq!(v.to_bits(), f64::from_be_bytes(bytes).to_bits());
            assert_eq!(c.data.len(), len as usize - 8);
        }
        Err(e) => {
            assert!(len < 8);
            assert_eq!(e, EtfError::UnexpectedEof);
        }
    }
}

#[kani::proof]
#[kani::unwind(5)]
fn cursor_take() {
    let bytes: [u8; 4] = kani::any();
    let len: u8 = kani::any();
    let n: u8 = kani::any();
    kani::assume(len <= 4);
    let data = &bytes[..len as usize];
    let mut c = Cursor::new(data);
    match c.take(n as usize) {
        Ok(s) => {
            assert_eq!(s.len(), n as usize);
            for i in 0..n as usize {
                assert_eq!(s[i], bytes[i]);
            }
            assert_eq!(c.data.len(), len as usize - n as usize);
        }
        Err(e) => {
            assert!(len < n);
            assert_eq!(e, EtfError::UnexpectedEof);
        }
    }
}

#[kani::proof]
fn cursor_take_streaming() {
    let bytes: [u8; 4] = kani::any();
    let len: u8 = kani::any();
    let n: u8 = kani::any();
    kani::assume(len <= 4);
    let data = &bytes[..len as usize];
    let mut c = Cursor::new_streaming(data);
    match c.take(n as usize) {
        Ok(s) => assert_eq!(s.len(), n as usize),
        Err(e) => {
            assert!(len < n);
            assert_eq!(e, EtfError::Incomplete(Needed::Size(n as usize)));
        }
    }
}

#[kani::proof]
#[kani::unwind(9)]
fn cursor_consumed_and_slice_between() {
    let bytes: [u8; 8] = kani::any();
    let len: u8 = kani::any();
    let start: u8 = kani::any();
    let end: u8 = kani::any();
    kani::assume(len <= 8);
    kani::assume(start <= end && end <= len);
    let data = &bytes[..len as usize];
    let mut c = Cursor::new(data);
    let _ = c.read_u8();
    assert_eq!(c.consumed(), 1);
    let s = c.slice_between(start as usize, end as usize);
    assert_eq!(s.len(), end as usize - start as usize);
    for i in 0..s.len() {
        assert_eq!(s[i], bytes[start as usize + i]);
    }
}
