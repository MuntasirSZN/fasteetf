use crate::error::{EtfError, Needed};

/// A zero-copy cursor over the unconsumed input bytes.
///
/// Keeps a reference to the *original* slice so that any consumed range can
/// later be recovered via [`slice_between`].
pub(crate) struct Cursor<'a> {
    /// The original input (kept for sub-slice recovery).
    original: &'a [u8],
    /// Remaining unconsumed portion of `original`.
    pub(crate) data: &'a [u8],
    /// When `true`, short reads return [`Incomplete`] instead of
    /// [`UnexpectedEof`].
    streaming: bool,
}

impl<'a> Cursor<'a> {
    /// Create a new cursor over `data` (non-streaming).
    #[inline(always)]
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Cursor {
            original: data,
            data,
            streaming: false,
        }
    }

    /// Create a new cursor in **streaming** mode.
    #[inline(always)]
    pub(crate) fn new_streaming(data: &'a [u8]) -> Self {
        Cursor {
            original: data,
            data,
            streaming: true,
        }
    }

    /// Number of bytes consumed from the original input so far.
    #[inline(always)]
    pub(crate) fn consumed(&self) -> usize {
        self.original.len() - self.data.len()
    }

    /// Return the sub-slice of the *original* input between two consumed
    /// offsets (`start` … `end`).
    #[inline(always)]
    pub(crate) fn slice_between(&self, start: usize, end: usize) -> &'a [u8] {
        &self.original[start..end]
    }

    /// Return the number of bytes that would be needed to make progress,
    /// or [`UnexpectedEof`] if not in streaming mode.
    #[inline(always)]
    fn eof_or_incomplete(&self, needed: usize) -> EtfError {
        if self.streaming {
            EtfError::Incomplete(Needed::Size(needed))
        } else {
            EtfError::UnexpectedEof
        }
    }

    /// Consume exactly `n` bytes from the front and return them.
    #[inline(always)]
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], EtfError> {
        if self.data.len() < n {
            return Err(self.eof_or_incomplete(n));
        }
        let (head, tail) = self.data.split_at(n);
        self.data = tail;
        Ok(head)
    }

    #[inline(always)]
    pub(crate) fn read_u8(&mut self) -> Result<u8, EtfError> {
        let (&b, rest) = self
            .data
            .split_first()
            .ok_or_else(|| self.eof_or_incomplete(1))?;
        self.data = rest;
        Ok(b)
    }

    #[inline(always)]
    pub(crate) fn read_u16(&mut self) -> Result<u16, EtfError> {
        if self.data.len() < 2 {
            return Err(self.eof_or_incomplete(2));
        }
        let val = u16::from_be_bytes([self.data[0], self.data[1]]);
        self.data = &self.data[2..];
        Ok(val)
    }

    #[inline(always)]
    pub(crate) fn read_u32(&mut self) -> Result<u32, EtfError> {
        if self.data.len() < 4 {
            return Err(self.eof_or_incomplete(4));
        }
        let val = u32::from_be_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]);
        self.data = &self.data[4..];
        Ok(val)
    }

    #[inline(always)]
    pub(crate) fn read_f64(&mut self) -> Result<f64, EtfError> {
        if self.data.len() < 8 {
            return Err(self.eof_or_incomplete(8));
        }
        let val = f64::from_be_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
            self.data[4],
            self.data[5],
            self.data[6],
            self.data[7],
        ]);
        self.data = &self.data[8..];
        Ok(val)
    }
}
