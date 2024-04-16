use std::io::Read;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};

use crate::unity::util::Endian;

pub trait ReadExt {
    fn read_string(&mut self, len: usize) -> anyhow::Result<String>;

    fn read_size<T: ByteOrder>(&mut self, size_bits: u32) -> anyhow::Result<usize>;

    fn read_dyn_string(&mut self, endian: &Endian, size_bits: u32) -> anyhow::Result<String>;
}

impl<R: Read + ?Sized> ReadExt for R {
    fn read_string(&mut self, len: usize) -> anyhow::Result<String> {
        let mut buf = vec![0; len];
        self.read_exact(&mut buf)
            .map_err(|e| anyhow::anyhow!("Failed to read string: {}", e))?;
        String::from_utf8(buf)
            .map_err(|e| anyhow::anyhow!("Failed to convert string to utf8: {}", e))
    }

    fn read_size<T: ByteOrder>(&mut self, size_bits: u32) -> anyhow::Result<usize> {
        match size_bits {
            8 => self.read_u8().map(|v| v as usize),
            16 => self.read_u16::<T>().map(|v| v as usize),
            32 => self.read_u32::<T>().map(|v| v as usize),
            _ => return Err(anyhow::anyhow!("Invalid size bits: {}", size_bits)),
        }.map_err(|e| anyhow::anyhow!("Failed to read size: {}", e))
    }

    fn read_dyn_string(&mut self, endian: &Endian, size_bits: u32) -> anyhow::Result<String> {
        let size = match endian {
            Endian::Little => self.read_size::<LittleEndian>(size_bits),
            Endian::Big => self.read_size::<BigEndian>(size_bits),
        }?;

        self.read_string(size)
    }
}