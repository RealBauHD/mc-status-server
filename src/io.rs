use std::io::{Error, ErrorKind, Read, Write};
use std::string::FromUtf8Error;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub fn read_var_int(read: &mut &[u8]) -> Result<i32, Error> {
    let mut value = 0;
    for i in 0..5 {
        let byte = read.read_u8()?;
        value |= (byte as i32 & 0x7F) << (i * 7);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(Error::new(ErrorKind::Other, "VarInt too large!"))
}

pub fn read_string(read: &mut &[u8]) -> Result<String, FromUtf8Error> {
    let length = read_var_int(read).unwrap();
    let mut buf = vec![0u8; length as usize];
    read.read_exact(&mut buf).expect("Error while decoding string");
    String::from_utf8(buf)
}

pub fn write_var_int(mut write: impl Write, value: i32) {
    if value & (0xFFFFFFFFu32 << 7) as i32 == 0 {
        write.write_i8(value as i8).unwrap();
    } else if (value & (0xFFFFFFFFu32 << 14) as i32) == 0 {
        write.write_i16::<BigEndian>(((value & 0x7F | 0x80) << 8 | (value >> 7)) as i16).unwrap();
    } else if (value & (0xFFFFFFFFu32 << 21) as i32) == 0 {
        write.write_i24::<BigEndian>((value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14)).unwrap();
    } else if (value & (0xFFFFFFFFu32 << 28) as i32) == 0 {
        write.write_i32::<BigEndian>((value & 0x7F | 0x80) << 24 | (((value >> 7) & 0x7F | 0x80) << 16)
            | ((value >> 14) & 0x7F | 0x80) << 8 | (value >> 21)).unwrap();
    } else {
        write.write_i32::<BigEndian>((value & 0x7F | 0x80) << 24 | ((value >> 7) & 0x7F | 0x80) << 16
            | ((value >> 14) & 0x7F | 0x80) << 8 | ((value >> 21) & 0x7F | 0x80)).unwrap();
        write.write_i8((value >> 28) as i8).unwrap();
    }
}

pub fn write_string(mut write: impl Write, s: String) {
    write_var_int(&mut write, s.len() as i32);
    write.write(s.as_bytes()).unwrap();
}

pub fn size_in_bytes(i: i32) -> usize {
    match i {
        0 => 1,
        n => (31 - n.leading_zeros() as usize) / 7 + 1,
    }
}