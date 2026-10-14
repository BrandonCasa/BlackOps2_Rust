use crate::core::*;
use half::f16;
use serde_json::{Value, json};

pub const MAGIC: &[u8; 12] = b"\xabKTX 20\xbb\r\n\x1a\n";
#[derive(Clone)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub faces: usize,
    pub format: u8,
    pub flags: u8,
    pub levels: Vec<Vec<u8>>,
}

pub fn sample(
    pixels: &[[f32; 4]],
    width: usize,
    height: usize,
    uv: [f32; 2],
    sampler: u8,
) -> [f32; 4] {
    let at = |x: i64, y: i64| {
        let x = if sampler & 32 != 0 {
            x.clamp(0, width as i64 - 1)
        } else {
            x.rem_euclid(width as i64)
        } as usize;
        let y = if sampler & 64 != 0 {
            y.clamp(0, height as i64 - 1)
        } else {
            y.rem_euclid(height as i64)
        } as usize;
        pixels[y * width + x]
    };
    let u = if sampler & 32 != 0 {
        uv[0].clamp(0., 1.)
    } else {
        uv[0].rem_euclid(1.)
    };
    let v = if sampler & 64 != 0 {
        uv[1].clamp(0., 1.)
    } else {
        uv[1].rem_euclid(1.)
    };
    let (x, y) = (u * width as f32 - 0.5, v * height as f32 - 0.5);
    if sampler & 7 == 1 {
        return at((x + 0.5).floor() as i64, (y + 0.5).floor() as i64);
    }
    let (ix, iy, fx, fy) = (
        x.floor() as i64,
        y.floor() as i64,
        x - x.floor(),
        y - y.floor(),
    );
    let p = [
        at(ix, iy),
        at(ix + 1, iy),
        at(ix, iy + 1),
        at(ix + 1, iy + 1),
    ];
    std::array::from_fn(|c| {
        (p[0][c] * (1. - fx) + p[1][c] * fx) * (1. - fy) + (p[2][c] * (1. - fx) + p[3][c] * fx) * fy
    })
}

pub fn pixels(image: &Image, level: usize) -> Result<Vec<[f32; 4]>> {
    let width = (image.width >> level).max(1);
    let height = (image.height >> level).max(1);
    let slices = (image.depth >> level).max(1) * image.faces;
    let bytes = image
        .levels
        .get(level)
        .ok_or("Native image mip is absent")?;
    let (_, _, bw, bh, stride, _) = format(image.format)?;
    if bytes.len() != width.div_ceil(bw) * height.div_ceil(bh) * slices * stride {
        return Err("Native image mip has invalid byte length".into());
    }
    if bw == 1 {
        return bytes
            .chunks_exact(stride)
            .map(|pixel| {
                Ok(match image.format {
                    1 => pixel
                        .try_into()
                        .map(|values: [u8; 4]| values.map(|v| v as f32 / 255.))?,
                    3 => [
                        pixel[0] as f32 / 255.,
                        pixel[0] as f32 / 255.,
                        pixel[0] as f32 / 255.,
                        pixel[1] as f32 / 255.,
                    ],
                    4 => [
                        pixel[0] as f32 / 255.,
                        pixel[0] as f32 / 255.,
                        pixel[0] as f32 / 255.,
                        1.,
                    ],
                    5 => [0., 0., 0., pixel[0] as f32 / 255.],
                    19 => std::array::from_fn(|i| f16::from_bits(u16_at(pixel, i * 2)).to_f32()),
                    _ => return Err("Native uncompressed image decoding is not implemented".into()),
                })
            })
            .collect();
    }
    fn alpha(block: &[u8]) -> [f32; 16] {
        let a = block[0] as f32 / 255.;
        let b = block[1] as f32 / 255.;
        let mut palette = [a, b, 0., 0., 0., 0., 0., 1.];
        if block[0] > block[1] {
            for i in 1..=6 {
                palette[i + 1] = ((7 - i) as f32 * a + i as f32 * b) / 7.;
            }
        } else {
            for i in 1..=4 {
                palette[i + 1] = ((5 - i) as f32 * a + i as f32 * b) / 5.;
            }
        }
        let bits = block[2..8]
            .iter()
            .enumerate()
            .fold(0_u64, |value, (i, b)| value | (*b as u64) << (i * 8));
        std::array::from_fn(|i| palette[(bits >> (i * 3) & 7) as usize])
    }
    fn color(block: &[u8], bc1: bool) -> [[f32; 4]; 16] {
        let first = u16_at(block, 0);
        let second = u16_at(block, 2);
        let rgb = |value: u16| {
            [
                ((value >> 11) & 31) as f32 / 31.,
                ((value >> 5) & 63) as f32 / 63.,
                (value & 31) as f32 / 31.,
                1.,
            ]
        };
        let a = rgb(first);
        let b = rgb(second);
        let mut palette = [a, b, [0.; 4], [0.; 4]];
        if first > second || !bc1 {
            for c in 0..4 {
                palette[2][c] = (2. * a[c] + b[c]) / 3.;
                palette[3][c] = (a[c] + 2. * b[c]) / 3.;
            }
        } else {
            for c in 0..4 {
                palette[2][c] = (a[c] + b[c]) / 2.;
            }
        }
        let bits = u32_at(block, 4);
        std::array::from_fn(|i| palette[(bits >> (i * 2) & 3) as usize])
    }
    let mut output = vec![[0.; 4]; width * height * slices];
    for (index, block) in bytes.chunks_exact(stride).enumerate() {
        let decoded = match image.format {
            11 => color(block, true),
            12 => {
                let mut pixels = color(&block[8..], false);
                let bits = u64_at(block, 0);
                for i in 0..16 {
                    pixels[i][3] = (bits >> (i * 4) & 15) as f32 / 15.;
                }
                pixels
            }
            13 => {
                let mut pixels = color(&block[8..], false);
                let values = alpha(block);
                for i in 0..16 {
                    pixels[i][3] = values[i];
                }
                pixels
            }
            14 => {
                let red = alpha(block);
                let green = alpha(&block[8..]);
                std::array::from_fn(|i| [red[i], green[i], 0., 1.])
            }
            _ => {
                return Err(format!(
                    "Native compressed image decoding is not implemented for {}",
                    image.format
                )
                .into());
            }
        };
        let columns = width.div_ceil(4);
        let blocks_per_slice = columns * height.div_ceil(4);
        let slice = index / blocks_per_slice;
        let x = index % blocks_per_slice % columns * 4;
        let y = index % blocks_per_slice / columns * 4;
        for row in 0..4 {
            for column in 0..4 {
                if x + column < width && y + row < height {
                    output[slice * width * height + (y + row) * width + x + column] =
                        decoded[row * 4 + column];
                }
            }
        }
    }
    Ok(output)
}
pub fn format(id: u8) -> Result<(u32, u32, usize, usize, usize, u8)> {
    Ok(match id {
        1 => (37, 43, 1, 1, 4, 1),
        3 => (16, 22, 1, 1, 2, 1),
        4 | 5 => (9, 15, 1, 1, 1, 1),
        11 => (133, 134, 4, 4, 8, 128),
        12 => (135, 136, 4, 4, 16, 129),
        13 => (137, 138, 4, 4, 16, 130),
        14 => (141, 141, 4, 4, 16, 132),
        19 => (97, 97, 1, 1, 8, 1),
        21 => (145, 146, 4, 4, 16, 134),
        22 => (139, 139, 4, 4, 8, 131),
        23 => (143, 143, 4, 4, 16, 133),
        _ => return Err(format!("Unsupported IWI encoding {id}").into()),
    })
}
pub fn read(bytes: &[u8]) -> Result<Image> {
    if bytes.len() < 64 || &bytes[..4] != b"IWi\x1b" {
        return Err("Expected original T6 IWI v27".into());
    }
    let id = bytes[4];
    let flags = bytes[5];
    let width = u16_at(&bytes, 6) as usize;
    let height = u16_at(&bytes, 8) as usize;
    let depth = u16_at(&bytes, 10) as usize;
    let faces = if flags & 4 != 0 { 6 } else { 1 };
    let (_, _, bw, bh, bs, _) = format(id)?;
    if width == 0
        || height == 0
        || depth > 1 && flags & 8 == 0
        || faces == 6 && (width != height || depth > 1)
    {
        return Err("Invalid native texture dimensions".into());
    }
    let count = if flags & 2 != 0 {
        1
    } else {
        usize::BITS - width.max(height).max(depth).leading_zeros()
    } as usize;
    let mut levels = vec![Vec::new(); count];
    let mut cursor = 64;
    for level in (0..count).rev() {
        let size = (width >> level).max(1).div_ceil(bw)
            * (height >> level).max(1).div_ceil(bh)
            * (depth >> level).max(1)
            * faces
            * bs;
        let end = cursor + size;
        if end > bytes.len() {
            return Err(format!("Truncated IWI mip {level}").into());
        }
        if level < 8 {
            let stored = u32_at(&bytes, 32 + level * 4) as usize;
            if stored != 0 && stored != end {
                return Err("IWI picmip offset disagrees".into());
            }
        }
        levels[level] = bytes[cursor..end].to_vec();
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err("Unexpected trailing IWI data".into());
    }
    Ok(Image {
        width,
        height,
        depth,
        faces,
        format: id,
        flags,
        levels,
    })
}

mod encode;
pub use encode::{encode, ktx2, validate};

#[cfg(test)]
mod tests {
    use super::*;
    use ctt_intel_texture_compressor::{
        self as encoder, RSurface, RgSurface, RgbaF16Surface, RgbaSurface,
    };

    #[test]
    fn worker_compression_matches_whole_surface_blocks() {
        for (role, encoding, hdr, opaque) in [
            ("normal", 14, false, true),
            ("mask", 22, false, true),
            ("color", 21, false, true),
            ("response", 21, false, false),
            ("sky", 23, true, true),
        ] {
            for (width, height, faces) in [(4, 4, 1), (28, 76, 1), (68, 12, 6)] {
                let mut source = Image {
                    width,
                    height,
                    faces,
                    depth: 1,
                    flags: 0,
                    format: if hdr { 19 } else { 1 },
                    levels: Vec::new(),
                };
                let (mut w, mut h) = (width, height);
                let mut expected = Vec::new();
                loop {
                    let stride = if hdr { 8 } else { 4 };
                    let mut data = Vec::new();
                    for index in 0..w * h * faces {
                        if hdr {
                            for value in [
                                ((index * 13) % 71) as f32 * 0.125,
                                ((index * 7) % 67) as f32 * 0.2,
                                ((index * 19) % 59) as f32 * 0.25,
                                1.,
                            ] {
                                data.extend(f16::from_f32(value).to_le_bytes());
                            }
                        } else {
                            data.extend([
                                (index * 13) as u8,
                                (index * 7) as u8,
                                (index * 19) as u8,
                                if opaque { 255 } else { (index * 11) as u8 },
                            ]);
                        }
                    }
                    let (pw, ph) = (w.next_multiple_of(4), h.next_multiple_of(4));
                    let components = match encoding {
                        14 => 2,
                        22 => 1,
                        _ => stride,
                    };
                    let mut blocks = Vec::new();
                    for face in data.chunks_exact(w * h * stride) {
                        let padded: Vec<_> = (0..ph)
                            .flat_map(|y| {
                                (0..pw).flat_map(move |x| {
                                    let start = (y.min(h - 1) * w + x.min(w - 1)) * stride;
                                    face[start..start + components].iter().copied()
                                })
                            })
                            .collect();
                        let (pw, ph, row) = (pw as u32, ph as u32, (pw * components) as u32);
                        blocks.extend(match encoding {
                            14 => {
                                encoder::bc5::compress_blocks(&RgSurface::new(&padded, pw, ph, row))
                            }
                            22 => {
                                encoder::bc4::compress_blocks(&RSurface::new(&padded, pw, ph, row))
                            }
                            23 => encoder::bc6h::compress_blocks(
                                &encoder::bc6h::basic_settings(),
                                &RgbaF16Surface::new(&padded, pw, ph, row),
                            ),
                            _ => encoder::bc7::compress_blocks(
                                &if opaque {
                                    encoder::bc7::opaque_basic_settings()
                                } else {
                                    encoder::bc7::alpha_basic_settings()
                                },
                                &RgbaSurface::new(&padded, pw, ph, row),
                            ),
                        });
                    }
                    source.levels.push(data);
                    expected.push(blocks);
                    if w == 1 && h == 1 {
                        break;
                    }
                    (w, h) = ((w / 2).max(1), (h / 2).max(1));
                }
                let bytes = encode(&source, role == "color", role).unwrap();
                validate(&bytes).unwrap();
                for (level, expected) in expected.iter().enumerate() {
                    let offset = u64_at(&bytes, 80 + level * 24) as usize;
                    let length = u64_at(&bytes, 88 + level * 24) as usize;
                    assert_eq!(
                        &bytes[offset..offset + length],
                        expected,
                        "{role}, {width}x{height}, {faces} faces, mip {level}"
                    );
                }
            }
        }
    }
}
