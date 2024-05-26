use std::io::{Seek, Write};

use anyhow::Context;
use binrw::__private::write_zeroes;
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};

use crate::unity::util::Endian;

pub trait WriteExt {
    fn align(&mut self, alignment: u64) -> anyhow::Result<()>;

    fn write_u32_order(&mut self, endian: &Endian, val: u32) -> anyhow::Result<()>;

    fn write_dyn_string(&mut self, s: &str, endian: &Endian) -> anyhow::Result<()>;
}

impl<W: Write + Seek> WriteExt for W {
    fn align(&mut self, alignment: u64) -> anyhow::Result<()> {
        let pos = self.stream_position()?;
        let rem = pos % alignment;
        if rem != 0 {
            let pad = alignment - rem;
            write_zeroes(self, pad).context("Failed to align")?;
        }
        Ok(())
    }

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
        self.write_all(s.as_bytes()).context("Failed to write string")?;
        self.align(4)
    }
}