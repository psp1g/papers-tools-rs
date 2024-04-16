use std::io::{Read, Seek, Write};
use std::ops::Deref;

use binrw::{BinRead, BinResult, binrw, BinWrite, Endian as BinrwEndian};

#[derive(Debug, PartialEq, Clone)]
pub struct U8Bool(pub bool);

impl Deref for U8Bool {
    type Target = bool;

    fn deref(&self) -> &bool {
        &self.0
    }
}

impl BinRead for U8Bool {
    type Args<'a> = ();

    fn read_options<R: Read + Seek>(reader: &mut R, endian: BinrwEndian, _: Self::Args<'_>) -> BinResult<Self> {
        let val = <u8>::read_options(reader, endian, ())?;
        Ok(U8Bool(val != 0))
    }
}

impl BinWrite for U8Bool {
    type Args<'a> = ();

    fn write_options<W: Write + Seek>(&self, writer: &mut W, _endian: BinrwEndian, _args: Self::Args<'_>) -> BinResult<()> {
        let buf = if **self { [1u8; 1] } else { [0u8; 1] };
        writer.write_all(&buf)?;

        Ok(())
    }
}

#[binrw]
#[brw(repr = u8)]
#[derive(Debug, Eq, PartialEq, Clone)]
#[repr(u8)]
pub enum Endian {
    Little,
    Big,
}
