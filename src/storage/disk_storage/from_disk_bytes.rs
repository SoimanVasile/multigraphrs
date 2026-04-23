pub trait FromDiskBytes {
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl FromDiskBytes for String{
    fn from_bytes(bytes: &[u8]) -> Self {
        std::str::from_utf8(bytes).unwrap().to_string()
    }
}

macro_rules! impl_from_disk_bytes_numeric {
    ($($t:ty),*) => {
        $(
            impl FromDiskBytes for $t {
                fn from_bytes(bytes: &[u8]) -> Self {
                    let arr = bytes
                        .try_into()
                        .expect("Fatal: Corrupted disk read! Byte slice length mismatch!");
                    <$t>::from_le_bytes(arr)
                }
            }
        )*
    };
}

impl_from_disk_bytes_numeric!(
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64
);

