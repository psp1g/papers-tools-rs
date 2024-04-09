use std::io::{Cursor, Read};

pub trait ReadExt {
    fn read_i16(&mut self) -> i16;
    
    fn read_u64(&mut self) -> u64;

    fn read_string(&mut self) -> String;
}

impl<T: AsRef<[u8]>> ReadExt for Cursor<T> {
    fn read_i16(&mut self) -> i16 {
        let mut buf = [0; 2];
        self.read_exact(&mut buf).unwrap();
        i16::from_le_bytes(buf)
    }

    fn read_u64(&mut self) -> u64 {
        let mut buf = [0; 8];
        self.read_exact(&mut buf).unwrap();
        u64::from_le_bytes(buf)
    }

    fn read_string(&mut self) -> String {
        let len = self.read_i16() as usize;
        let mut buf = vec![0; len];
        self.read_exact(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }
}