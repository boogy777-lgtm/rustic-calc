use std::fs::File;
use std::io::{self, Write};

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                0xedb8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    crc ^ 0xffff_ffff
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

fn append_chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let mut checksum = Vec::with_capacity(kind.len() + data.len());
    checksum.extend_from_slice(kind);
    checksum.extend_from_slice(data);
    output.extend_from_slice(&crc32(&checksum).to_be_bytes());
}

fn zlib_uncompressed(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() + data.len() / 65_535 * 5 + 16);
    output.extend_from_slice(&[0x78, 0x01]);
    let mut offset = 0;
    while offset < data.len() {
        let length = (data.len() - offset).min(65_535);
        let is_last = offset + length == data.len();
        output.push(u8::from(is_last));
        output.extend_from_slice(&(length as u16).to_le_bytes());
        output.extend_from_slice(&(!(length as u16)).to_le_bytes());
        output.extend_from_slice(&data[offset..offset + length]);
        offset += length;
    }
    output.extend_from_slice(&adler32(data).to_be_bytes());
    output
}

/// Writes a top-down BGRA buffer as an 8-bit RGB PNG.
pub fn write_bgra(path: &str, width: u32, height: u32, pixels: &[u8]) -> io::Result<()> {
    let stride = width as usize * 4;
    let expected = stride
        .checked_mul(height as usize)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "image dimensions overflow"))?;
    if pixels.len() != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "pixel buffer size does not match image dimensions",
        ));
    }

    let mut raw = Vec::with_capacity((stride / 4 * 3 + 1) * height as usize);
    for row in pixels.chunks_exact(stride).take(height as usize) {
        raw.push(0);
        for pixel in row.chunks_exact(4) {
            raw.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
    }

    let mut output = Vec::with_capacity(raw.len() + 128);
    output.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    append_chunk(&mut output, b"IHDR", &header);
    append_chunk(&mut output, b"IDAT", &zlib_uncompressed(&raw));
    append_chunk(&mut output, b"IEND", &[]);

    File::create(path)?.write_all(&output)
}

#[cfg(test)]
mod tests {
    use super::{adler32, crc32, zlib_uncompressed};

    #[test]
    fn png_checksums_match_standard_vectors() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn zlib_stream_has_header_and_adler_trailer() {
        let stream = zlib_uncompressed(b"slint");
        assert_eq!(&stream[..2], &[0x78, 0x01]);
        assert_eq!(
            &stream[stream.len() - 4..],
            &adler32(b"slint").to_be_bytes()
        );
    }
}
