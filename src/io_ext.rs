use std::io::{Read, Write};
use anyhow::Context;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::unity::util::Endian;

pub trait ReadExt {
    fn read_string(&mut self, len: usize) -> anyhow::Result<String>;

    fn read_u32_order(&mut self, endian: &Endian) -> anyhow::Result<u32>;

    fn read_size<T: ByteOrder>(&mut self, size_bits: u32) -> anyhow::Result<usize>;

    fn read_dyn_string(&mut self, endian: &Endian) -> anyhow::Result<String>;
}

impl<R: Read + ?Sized> ReadExt for R {
    fn read_string(&mut self, len: usize) -> anyhow::Result<String> {
        let mut buf = vec![0; len];
        self.read_exact(&mut buf)
            .context("Failed to read string")?;
        String::from_utf8(buf)
            .context("Failed to convert string to utf8")
    }

    fn read_u32_order(&mut self, endian: &Endian) -> anyhow::Result<u32> {
        match endian {
            Endian::Little => self.read_u32::<LittleEndian>(),
            Endian::Big => self.read_u32::<BigEndian>(),
        }.context("Failed to read u32")
    }

    fn read_size<T: ByteOrder>(&mut self, size_bits: u32) -> anyhow::Result<usize> {
        match size_bits {
            8 => self.read_u8().map(|v| v as usize),
            16 => self.read_u16::<T>().map(|v| v as usize),
            32 => self.read_u32::<T>().map(|v| v as usize),
            _ => return Err(anyhow::anyhow!("Invalid size bits: {}", size_bits)),
        }.context("Failed to read size")
    }

    fn read_dyn_string(&mut self, endian: &Endian) -> anyhow::Result<String> {
        let size = match endian {
            Endian::Little => self.read_u32::<LittleEndian>(),
            Endian::Big => self.read_u32::<BigEndian>(),
        }? as usize;

        self.read_string(size)
    }
}

pub trait WriteExt {

    fn write_u32_order(&mut self, endian: &Endian, val: u32) -> anyhow::Result<()>;

    fn write_dyn_string(&mut self, s: &str, endian: &Endian) -> anyhow::Result<()>;

}

impl<W: Write + ?Sized> WriteExt for W {
    fn write_u32_order(&mut self, endian: &Endian, val: u32) -> anyhow::Result<()> {
        match endian {
            Endian::Little => self.write_u32::<LittleEndian>(val),
            Endian::Big => self.write_u32::<BigEndian>(val),
        }.context("Failed to write u32")
    }

    fn write_dyn_string(&mut self, s: &str, endian: &Endian) -> anyhow::Result<()> {
        let size = s.len() as u32;
        match endian {
            Endian::Little => self.write_u32::<LittleEndian>(size),
            Endian::Big => self.write_u32::<BigEndian>(size),
        }?;
        self.write_all(s.as_bytes())
            .context("Failed to write string")
    }
}