use multigraphrs::storage::disk_storage::from_disk_bytes::FromDiskBytes;

#[test]
fn test_from_disk_bytes() {
    let bytes: &[u8] = &[1, 0, 0, 0];
    let val = u32::from_bytes(bytes);
    assert_eq!(val, 1);

    let bytes_f32: &[u8] = &[0, 0, 128, 63]; // 1.0f32 in little-endian
    let val_f32 = f32::from_bytes(bytes_f32);
    assert_eq!(val_f32, 1.0);

    let bytes_f64: &[u8] = &[0, 0, 0, 0, 0, 0, 240, 63]; // 1.0f64 in little-endian
    let val_f64 = f64::from_bytes(bytes_f64);
    assert_eq!(val_f64, 1.0);
}

#[test]
#[should_panic(expected = "Fatal: Corrupted disk read! Byte slice length mismatch!")]
fn test_from_disk_bytes_invalid_length() {
    let bytes: &[u8] = &[1, 0, 0]; // 3 bytes instead of 4 for u32
    let _ = u32::from_bytes(bytes);
}

#[test]
fn test_from_disk_bytes_string() {
    let bytes: &[u8] = b"hello";
    let val = String::from_bytes(bytes);
    assert_eq!(val, "hello");
}

#[test]
#[should_panic]
fn test_from_disk_bytes_string_invalid() {
    let bytes: &[u8] = &[0xFF, 0xFF, 0xFF];
    let _ = String::from_bytes(bytes);
}
