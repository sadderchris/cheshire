use core::fmt;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

use gc_arena::{static_collect, Collect};

use crate::vm::Result;

/// Input port
pub struct ObjReadPort {
    resource: BufReader<Box<dyn Read>>,
}

static_collect!(ObjReadPort);

impl ObjReadPort {
    /// Construct a ObjReadPort
    pub fn new<R: Read + 'static>(reader: R) -> Self {
        Self {
            resource: BufReader::new(Box::new(reader)),
        }
    }

    /// Read a character from the input
    pub fn read_char(&mut self) -> Result<Option<char>> {
        let result = self.peek_char()?;
        if let Some(result) = result {
            self.consume(result.len_utf8());
        }
        Ok(result)
    }

    /// Peek a character from the input
    pub fn peek_char(&mut self) -> Result<Option<char>> {
        let buf = self.fill_buf()?;
        let character = core::str::from_utf8(buf)?.chars().next();
        Ok(character)
    }

    /// Is a character ready from the input?
    pub fn is_char_ready(&self) -> bool {
        !self.resource.buffer().is_empty()
    }

    pub(crate) fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.resource.fill_buf()
    }

    pub(crate) fn consume(&mut self, size: usize) {
        self.resource.consume(size);
    }
}

impl fmt::Display for ObjReadPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#<input port {:p}>", self)
    }
}

// This is dumb, but it's better than redefining Read and Write traits
impl fmt::Debug for ObjReadPort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjReadPort")
            .field("resource", &(&self.resource as *const dyn Read))
            .finish()
    }
}

/// Output port
pub struct ObjWritePort {
    resource: BufWriter<Box<dyn Write>>,
}

static_collect!(ObjWritePort);

impl ObjWritePort {
    /// Construct a ObjWritePort
    pub fn new<W: Write + 'static>(writer: W) -> Self {
        Self {
            resource: BufWriter::new(Box::new(writer)),
        }
    }

    /// Write a single character to the write buffer
    pub fn write_char(&mut self, character: char) -> io::Result<usize> {
        let buf = &mut [0; 4];
        let result = character.encode_utf8(buf).len();
        let result = self.resource.write(&buf[0..result]);
        // TODO: fix this - this is pretty inefficient
        self.resource.flush()?;
        result
    }
}

// This is dumb, but it's better than redefining Read and Write traits
impl fmt::Debug for ObjWritePort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ObjWritePort")
            .field("resource", &(&self.resource as *const dyn Write))
            .finish()
    }
}

impl fmt::Display for ObjWritePort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#<output port {:p}>", self)
    }
}
