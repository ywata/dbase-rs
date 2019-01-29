use std::io::{Read, Write};
use std::str::FromStr;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use record::RecordFieldInfo;
use Error;


#[allow(dead_code)]
#[derive(Debug)]
pub enum FieldType {
    Character,
    Currency,
    Numeric,
    Float,
    Date,
    DateTime,
    Double,
    Integer,
    Logical,
    Memo,
    General,
    BinaryCharacter,
    BinaryMemo,
    Picture,
    Varbinary,
    BinaryVarchar,
}

impl FieldType {
    pub fn from(c: char) -> Option<FieldType> {
        match c {
            'C' => Some(FieldType::Character),
            'Y' => Some(FieldType::Currency),
            'N' => Some(FieldType::Numeric),
            'F' => Some(FieldType::Float),
            'D' => Some(FieldType::Date),
            'T' => Some(FieldType::DateTime),
            'B' => Some(FieldType::Double),
            'I' => Some(FieldType::Integer),
            'L' => Some(FieldType::Logical),
            'M' => Some(FieldType::Memo),
            'G' => Some(FieldType::General),
            //'C' => Some(FieldType::BinaryCharacter), ??
            //'M' => Some(FieldType::BinaryMemo),
            _ => None,
        }
    }

    pub fn try_from(c: char) -> Result<FieldType, Error> {
        match Self::from(c) {
            Some(t) => Ok(t),
            None => Err(Error::InvalidFieldType(c))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Date {
    year: u32,
    month: u32,
    day: u32,
}

impl Date {
    pub(crate) fn from_bytes(bytes: [u8; 3]) -> Self {
        Self {
            year: 1900u32 + bytes[0] as u32,
            month: bytes[1] as u32,
            day: bytes[2] as u32,
        }
    }

    pub(crate) fn write_to<T: Write>(&self, dest: &mut T) -> Result<(), Error> {
        self.validate()?;
        dest.write_u8((self.year - 1900) as u8)?;        
        dest.write_u8(self.month as u8)?;
        dest.write_u8(self.day as u8)?;
        Ok(())
    }

    // Does some extremely basic checks
    fn validate(&self) -> Result<(), Error> {
        if self.month > 12 ||
           self.day > 31 ||
           self.year < 1900 ||
           self.year > 2155 {
               Err(Error::InvalidDate)
           }
        else {
            Ok(())
        }
    }


}


impl FromStr for Date {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let year = s[0..4].parse::<u32>()?;
        let month = s[4..6].parse::<u32>()?;
        let day = s[6..8].parse::<u32>()?;

        Ok(Self {
            year,
            month,
            day,
        })
    }
}


/// Enum where each variant stores the record value
#[derive(Debug, PartialEq)]
pub enum FieldValue {
    Character(String),
    Numeric(f64),
    //Stored as String
    Logical(bool),
    // Stored as one char
    Integer(i32),
    Float(f32),
    Double(f64),
    Date(Date),
}

impl FieldValue {
    pub(crate) fn read_from<T: Read>(mut source: &mut T, field_info: &RecordFieldInfo) -> Result<Self, Error> {
        let value = match field_info.field_type {
            FieldType::Logical => {
                match source.read_u8()? as char {
                    '1' | 'T' | 't' | 'Y' | 'y' => FieldValue::Logical(true),
                    _ => FieldValue::Logical(false),
                }
            }
            FieldType::Integer => {
                FieldValue::Integer(source.read_i32::<LittleEndian>()?)
            }
            FieldType::Character => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Character(value.trim().to_owned())
            }
            FieldType::Numeric => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Numeric(value.trim().parse::<f64>()?)
            }
            FieldType::Float => FieldValue::Float(source.read_f32::<LittleEndian>()?),
            FieldType::Double => FieldValue::Double(source.read_f64::<LittleEndian>()?),
            FieldType::Date => {
                let value = read_string_of_len(&mut source, field_info.record_length)?;
                FieldValue::Date(value.parse::<Date>()?)
            }
            _ => panic!("unhandled type")
        };
        Ok(value)
    }
}

fn read_string_of_len<T: Read>(source: &mut T, len: u8) -> Result<String, std::io::Error> {
    let mut bytes = Vec::<u8>::new();
    bytes.resize(len as usize, 0u8);
    source.read_exact(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
