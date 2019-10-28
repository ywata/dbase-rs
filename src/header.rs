use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use record::field::Date;
use Error;

#[derive(Debug, Copy, Clone)]
pub enum Version {
    FoxBase,
    DBase3{has_memo: bool},
    Unknown,
}

impl Version {
    pub(crate) fn has_memo(&self) -> bool {
        match self {
             Version::FoxBase => false,
             Version::DBase3{has_memo} => *has_memo,
             _ => panic!("unknown version")
        }
    }
}

impl From<Version> for u8 {
    fn from(v: Version) -> u8 {
        match v {
             Version::FoxBase => 0x02,
             Version::DBase3{has_memo: false} => 0x03,
             Version::DBase3{has_memo: true} => 0x83,
             _ => panic!("unknown version")
        }
    }
}

impl From<u8> for Version {
    fn from(b: u8) -> Self {
        match b {
            0x02 => Version::FoxBase,
            0x03 => Version::DBase3{has_memo: false},
            0x83 => Version::DBase3{has_memo: true},
            _ => {
                println!("Unknown version byte: {}", b);
                Version::Unknown
            }
        }
    }
}


pub struct TableFlags(u8);

impl TableFlags {
    pub fn has_structural_cdx(&self) -> bool {
        (self.0 & 0x01) == 1
    }

    pub fn has_memo_field(&self) -> bool {
        (self.0 & 0x02) == 2
    }

    pub fn is_a_database(&self) -> bool {
        (self.0 & 0x03) == 1
    }
}


pub struct Header {
    pub file_type: Version,
    pub last_update: Date,
    pub num_records: u32,
    pub offset_to_first_record: u16,
    pub size_of_record: u16,
    pub is_transaction_incomplete: bool,
    pub encryption_flag: u8,
    pub table_flags: TableFlags,
    pub code_page_mark: u8, //FIXME is the "language driver id" instead ?
}


impl Header {
    pub(crate) fn new(num_records: u32, offset: u16, size_of_records: u16) -> Self {
        Self {
            file_type: Version::DBase3{has_memo: false},
            last_update: Date {
                year: 1990,
                month: 12,
                day: 25,
            }, //FIXME use chrono crate
            num_records,
            offset_to_first_record: offset,
            size_of_record: size_of_records,
            is_transaction_incomplete: false,
            encryption_flag: 0,
            table_flags: TableFlags { 0: 0 },
            code_page_mark: 0,
        }
    }

    pub(crate) const SIZE: usize = 32;

    pub(crate) fn read_from<T: Read>(source: &mut T) -> Result<Self, std::io::Error> {
        let file_type = Version::from(source.read_u8()?);

        let mut date = [0u8; 3];
        source.read_exact(&mut date)?;
        let last_update = Date::from_bytes(date);

        let num_records = source.read_u32::<LittleEndian>()?;
        let offset_to_first_record = source.read_u16::<LittleEndian>()?;
        let size_of_record = source.read_u16::<LittleEndian>()?;

        let _reserved = source.read_u16::<LittleEndian>()?;

        let is_transaction_incomplete = (source.read_u8()? != 0) as bool;
        let encryption_flag = source.read_u8()?;

        let mut _reserved = [0u8; 12];
        source.read_exact(&mut _reserved)?;

        let table_flags = TableFlags {
            0: source.read_u8()?,
        };

        let code_page_mark = source.read_u8()?;

        let _reserved = source.read_u8()?;
        let _reserved = source.read_u8()?;

        Ok(Self {
            file_type,
            last_update,
            num_records,
            offset_to_first_record,
            is_transaction_incomplete,
            encryption_flag,
            size_of_record,
            table_flags,
            code_page_mark,
        })
    }

    pub(crate) fn write_to<T: Write>(&self, mut dest: &mut T) -> Result<(), Error> {
        dest.write_u8(u8::from(self.file_type))?;
        self.last_update.write_to(&mut dest)?;
        dest.write_u32::<LittleEndian>(self.num_records)?;
        dest.write_u16::<LittleEndian>(self.offset_to_first_record)?;
        dest.write_u16::<LittleEndian>(self.size_of_record)?;

        // Reserved
        dest.write_u16::<LittleEndian>(0)?;

        let byte_value = if self.is_transaction_incomplete {
            1u8
        } else {
            0u8
        };
        dest.write_u8(byte_value)?;
        dest.write_u8(self.encryption_flag)?;

        let _reserved = [0u8; 12];
        dest.write_all(&_reserved)?;

        dest.write_u8(self.table_flags.0)?;
        dest.write_u8(self.code_page_mark)?;
        // Reserved
        dest.write_u8(0)?;
        dest.write_u8(0)?;
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use std::fs::File;

    use super::*;
    use std::io::{Cursor, Seek, SeekFrom};


    #[test]
    fn pos_after_reading_header() {
        let mut file = File::open("tests/data/line.dbf").unwrap();
        let _hdr = Header::read_from(&mut file).unwrap();
        let pos_after_reading = file.seek(SeekFrom::Current(0)).unwrap();
        assert_eq!(pos_after_reading, Header::SIZE as u64);
    }

    #[test]
    fn pos_after_writing_header() {
        let mut file = File::open("tests/data/line.dbf").unwrap();
        let hdr = Header::read_from(&mut file).unwrap();

        let mut out = Cursor::new(Vec::<u8>::with_capacity(Header::SIZE));
        hdr.write_to(&mut out).unwrap();
        let pos_after_writing = out.seek(SeekFrom::Current(0)).unwrap();
        assert_eq!(pos_after_writing, Header::SIZE as u64);
    }


    #[test]
    fn read_write_header() {
        let mut file = File::open("tests/data/line.dbf").unwrap();

        let mut hdr_bytes = [0u8; Header::SIZE];
        file.read_exact(&mut hdr_bytes).unwrap();
        let hdr_bytes: Vec<u8> = hdr_bytes.to_vec();

        let mut cursor = Cursor::new(hdr_bytes);
        let hdr = Header::read_from(&mut cursor).unwrap();
        let hdr_bytes = cursor.into_inner();

        let mut cursor = Cursor::new(Vec::<u8>::with_capacity(Header::SIZE));
        hdr.write_to(&mut cursor).unwrap();
        let hdr_bytes_written = cursor.into_inner();

        assert_eq!(hdr_bytes_written, hdr_bytes);
    }
}

