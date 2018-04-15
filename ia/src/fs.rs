use std::io::{Read, Write, Seek};
use std::io;
use error::IoResult;

pub struct File {
    handle: i32
}

impl File {
    pub fn open(path: &str, mode: &str) -> IoResult<File> {
        ::raw::file_open(path, mode).map(|h| {
            File {
                handle: h
            }
        })
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        ::raw::file_read(self.handle, buf)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                e
            ))
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        ::raw::file_write(self.handle, buf)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                e
            ))
    }

    fn flush(&mut self) -> io::Result<()> {
        ::raw::file_flush(self.handle)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                e
            ))
    }
}

impl Seek for File {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        ::raw::file_seek(self.handle, pos)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other,
                e
            ))
    }
}

impl Drop for File {
    fn drop(&mut self) {
        ::raw::file_close(self.handle);
    }
}
