use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::Arc,
};

use parking_lot::Mutex;

#[derive(Debug)]
pub struct Pipe {
    data: Arc<Mutex<VecDeque<u8>>>,
}
impl Pipe {
    #[inline]
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
    #[inline(always)]
    pub fn reader(&self) -> PipeReader {
        PipeReader {
            data: self.data.clone(),
        }
    }
    #[inline(always)]
    pub fn writer(&self) -> PipeWriter {
        PipeWriter {
            data: self.data.clone(),
        }
    }
}
impl Default for Pipe {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Debug)]
pub struct PipeReader {
    data: Arc<Mutex<VecDeque<u8>>>,
}
impl Read for PipeReader {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut data = self.data.lock();
        let len = buf.len().min(data.len());
        if len == 0 {
            return Ok(0);
        }
        for (i, byte) in data.drain(0..len).enumerate() {
            buf[i] = byte;
        }
        Ok(len)
    }
}
#[derive(Debug)]
pub struct PipeWriter {
    data: Arc<Mutex<VecDeque<u8>>>,
}
impl Write for PipeWriter {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut data = self.data.lock();
        data.extend(buf.iter());
        Ok(buf.len())
    }
    #[inline(always)]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
