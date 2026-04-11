use std::io;
use std::mem;

use crate::types;

// Data size is written as 64-bit bin-endian unsigned integer.
// This way data passed via disk/network can be correctly interpreted by other clients/servers.
type SizeT = u64;
const SIZE_LEN: usize = mem::size_of::<SizeT>();


macro_rules! impl_read_from_stream {
    ($($t:ty),*) => {
        $(
            impl types::Deserializable for $t {
                fn deserialize(stream: &mut dyn io::Read) -> types::Result<Self> {
                    const TYPE_SIZE: usize = mem::size_of::<$t>();
                    let mut buffer = [0u8; TYPE_SIZE];

                    let bytes_count = stream.read(&mut buffer)?;
                    if bytes_count != TYPE_SIZE {
                        return Err(
                            Box::new(
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("Not enough bytes to read {}", std::any::type_name::<$t>()),
                                ),
                            ),
                        );
                    }

                    Ok(<$t>::from_be_bytes(buffer))
                }
            }
        )*
    };
}

impl_read_from_stream!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);


impl<T: types::Deserializable> types::Deserializable for Option<T> {
    fn deserialize(stream: &mut dyn io::Read) -> types::Result<Option<T>> {
        let mut buffer = [0u8; 1];
        let bytes_count = stream.read(&mut buffer)?;
        if bytes_count != 1 {
            return Err(
                Box::new(
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Not enough bytes to read {}", std::any::type_name::<T>()),
                    ),
                ),
            );
        }

        let has_value = buffer[0] != 0;
        if has_value {
            let value = T::deserialize(stream)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}


impl types::Deserializable for String {
    fn deserialize(stream: &mut dyn io::Read) -> types::Result<Self> {
        let mut size_buffer = [0u8; SIZE_LEN];
        stream.read_exact(&mut size_buffer)?;
        let size = u64::from_be_bytes(size_buffer) as usize;

        let mut str_buffer = vec![0u8; size];
        str_buffer.reserve(size);
        stream.read_exact(&mut str_buffer[..])?;

        match String::from_utf8(str_buffer) {
            Ok(result) => Ok(result),
            Err(err) => {
                Err(
                    Box::new(
                        io::Error::new(io::ErrorKind::InvalidData, err.to_string())
                    )
                )
            }
        }
    }
}


macro_rules! impl_write_to_stream {
    ($($t:ty),*) => {
        $(
            impl types::Serializable for $t {
                fn serialize(&self, stream: &mut dyn io::Write) -> types::Result<()> {
                    let bytes = self.to_be_bytes();
                    stream.write(&bytes);
                    Ok(())
                }
            }
        )*
    };
}

impl_write_to_stream!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);


impl<T: types::Serializable> types::Serializable for Option<T> {
    fn serialize(&self, stream: &mut dyn io::Write) -> types::Result<()> {
        let expected_size = mem::size_of::<T>() + 1;
        let mut buffer = vec![];
        buffer.reserve(expected_size);
        
        let has_value = self.is_some();
        let bytes = if has_value {[1u8]} else {[0u8]};
        buffer.extend(bytes);

        if self.is_some() {
            self.as_ref().unwrap().serialize(&mut buffer)?;
        }

        stream.write(&buffer);
        Ok(())
    }
}


impl types::Serializable for String {
    fn serialize(&self, stream: &mut dyn io::Write) -> types::Result<()> {
        let expected_size = mem::size_of::<String>() + SIZE_LEN;
        let mut buffer = vec![];
        buffer.reserve(expected_size);

        let len = self.len() as u64;
        buffer.extend(len.to_be_bytes());
        buffer.extend(self.as_bytes());

        stream.write(&buffer)?;

        Ok(())
    }
}
