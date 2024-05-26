use std::io::{Read, Seek, Write};
use std::ops::Deref;

use binrw::{BinRead, BinResult, binrw, BinWrite, Endian as BinrwEndian, NamedArgs};
use binrw::__private::write_zeroes;

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

impl Into<BinrwEndian> for Endian {
    fn into(self) -> BinrwEndian {
        match self {
            Endian::Little => BinrwEndian::Little,
            Endian::Big => BinrwEndian::Big,
        }
    }
}

#[derive(NamedArgs, Clone)]
pub struct AlignmentArgs {
    alignment: u32,
}

impl AlignmentArgs {
    pub fn new(alignment: u32) -> Self {
        Self { alignment }
    }
}

impl Default for AlignmentArgs {
    fn default() -> Self {
        Self { alignment: 4 }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct AlignedString(pub String);

impl Deref for AlignedString {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl BinWrite for AlignedString {
    type Args<'a> = AlignmentArgs;

    fn write_options<W: Write + Seek>(&self, writer: &mut W, endian: BinrwEndian, args: Self::Args<'_>) -> BinResult<()> {
        // Write the string
        let len = self.0.len() as u32;
        len.write_options(writer, endian, ())?;
        writer.write_all(self.0.as_bytes())?;

        // Align the writer
        let pos = writer.stream_position()?;
        let align = args.alignment as u64;
        let rem = pos % align;
        if rem != 0 {
            write_zeroes(writer, align - rem)?;
        }

        Ok(())
    }
}

impl BinRead for AlignedString {
    type Args<'a> = AlignmentArgs;

    fn read_options<R: Read + Seek>(reader: &mut R, endian: BinrwEndian, args: Self::Args<'_>) -> BinResult<Self> {
        // Read the string
        let len = <u32>::read_options(reader, endian, ())? as usize;
        let mut buf = vec![0; len];
        reader.read_exact(&mut buf)?;
        let str = String::from_utf8(buf)
            .map(AlignedString)
            .map_err(|e| {
                let pos = reader.stream_position().unwrap();
                binrw::error::Error::AssertFail {
                    pos,
                    message: e.to_string(),
                }
            })?;

        // Align the reader
        let pos = reader.stream_position()?;
        let align = args.alignment as u64;
        let padding = (align - (pos % align)) % align;
        reader.seek(std::io::SeekFrom::Current(padding as i64))?;

        Ok(str)
    }

}