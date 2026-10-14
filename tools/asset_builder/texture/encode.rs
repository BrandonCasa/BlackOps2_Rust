use super::*;

fn descriptor(id: u8, srgb: bool) -> Result<Vec<u8>> {
    let (_, _, w, h, size, model) = format(id)?;
    let mut samples = Vec::new();
    let mut sample = |offset: u16, bits: u8, channel: u8, low: u32, high: u32| {
        push16(&mut samples, offset);
        samples.extend([bits - 1, channel, 0, 0, 0, 0]);
        push32(&mut samples, low);
        push32(&mut samples, high);
    };
    if model == 1 {
        let channels = if id == 1 || id == 19 {
            4
        } else if id == 3 {
            2
        } else {
            1
        };
        let bits = if id == 19 { 16 } else { 8 };
        for channel in 0..channels {
            let c = if channel == 3 || id == 5 || id == 3 && channel == 1 {
                15
            } else {
                channel
            };
            let qualifier = if id == 19 {
                0xc0
            } else if srgb && c == 15 {
                0x10
            } else {
                0
            };
            sample(
                channel as u16 * bits as u16,
                bits,
                c | qualifier,
                if id == 19 { 0xbf800000 } else { 0 },
                if id == 19 { 0x3f800000 } else { 255 },
            );
        }
    } else {
        let entries = if matches!(id, 21 | 23) {
            vec![(0, 128, 0)]
        } else if id == 22 {
            vec![(0, 64, 0)]
        } else if id == 11 {
            vec![(0, size * 8, 1)]
        } else if id == 12 || id == 13 {
            vec![(0, 64, 15), (64, 64, 0)]
        } else {
            vec![(0, 64, 0), (64, 64, 1)]
        };
        for (o, b, c) in entries {
            sample(
                o,
                b as u8,
                c | if id == 23 {
                    0x80
                } else if srgb && c == 15 {
                    0x10
                } else {
                    0
                },
                0,
                if id == 23 { 0x3f800000 } else { u32::MAX },
            );
        }
    }
    let mut bytes = Vec::new();
    push32(&mut bytes, (28 + samples.len()) as u32);
    push32(&mut bytes, 0);
    push16(&mut bytes, 2);
    push16(&mut bytes, (24 + samples.len()) as u16);
    bytes.extend([
        model,
        1,
        if srgb { 2 } else { 1 },
        0,
        (w - 1) as u8,
        (h - 1) as u8,
        0,
        0,
        size as u8,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);
    bytes.extend(samples);
    Ok(bytes)
}
pub fn encode(source: &Image, srgb: bool, role: &str) -> Result<Vec<u8>> {
    use ctt_intel_texture_compressor::{
        self as encoder, RSurface, RgSurface, RgbaF16Surface, RgbaSurface,
    };
    let srgb =
        srgb && !matches!(
            role,
            "normal" | "response" | "reflectance" | "occlusion" | "mask"
        ) && source.format != 19;
    if source.depth > 1
        || !matches!(source.format, 1 | 19)
        || !source.width.is_multiple_of(4)
        || !source.height.is_multiple_of(4)
    {
        return ktx2(source, srgb);
    }
    let encoding = if source.format == 19 {
        if !matches!(role, "color" | "sky" | "emissive" | "fx-color")
            || source.levels.iter().any(|level| {
                level.chunks_exact(8).any(|pixel| {
                    (0..3).any(|i| f16::from_bits(u16_at(pixel, i * 2)).to_f32() < 0.)
                        || f16::from_bits(u16_at(pixel, 6)).to_f32() != 1.
                })
            })
        {
            19
        } else {
            23
        }
    } else {
        match role {
            "normal" => 14,
            "occlusion" | "mask" => 22,
            "color" | "response" | "reflectance" | "emissive" | "fx-color" | "background"
            | "sky" => 21,
            _ => 1,
        }
    };
    if encoding == source.format {
        return ktx2(source, srgb);
    }
    let opaque = source.format == 1
        && source
            .levels
            .iter()
            .all(|level| level.chunks_exact(4).all(|pixel| pixel[3] == 255));
    let stride = if source.format == 19 { 8 } else { 4 };
    let mut compressed = source.clone();
    compressed.format = encoding;
    compressed.levels = source
        .levels
        .iter()
        .enumerate()
        .map(|(level, data)| -> Result<Vec<u8>> {
            let width = (source.width >> level).max(1);
            let height = (source.height >> level).max(1);
            let face_size = width * height * stride;
            if data.len() != face_size * source.faces {
                return Err("Invalid source face layout for compression".into());
            }
            let padded_width = width.next_multiple_of(4);
            let padded_height = height.next_multiple_of(4);
            let components = match encoding {
                14 => 2,
                22 => 1,
                _ => stride,
            };
            let mut result = Vec::new();
            for face in data.chunks_exact(face_size) {
                let mut padded = vec![0; padded_width * padded_height * components];
                parallel_chunks_mut(
                    &mut padded,
                    padded_width * components * 16,
                    |offset, padded| {
                        for (row, pixels) in padded
                            .chunks_exact_mut(padded_width * components)
                            .enumerate()
                        {
                            let y = offset / (padded_width * components) + row;
                            for x in 0..padded_width {
                                let src = (y.min(height - 1) * width + x.min(width - 1)) * stride;
                                let dst = x * components;
                                pixels[dst..dst + components]
                                    .copy_from_slice(&face[src..src + components]);
                            }
                        }
                        Ok(())
                    },
                )?;
                let block_size = if encoding == 22 { 8 } else { 16 };
                let block_row = padded_width / 4 * block_size;
                let mut blocks = vec![0; block_row * (padded_height / 4)];
                parallel_chunks_mut(&mut blocks, block_row * 4, |offset, blocks| {
                    let first_row = offset / block_row * 4;
                    let rows = blocks.len() / block_row * 4;
                    let pixels = &padded[first_row * padded_width * components
                        ..(first_row + rows) * padded_width * components];
                    let (w, h, row) = (
                        padded_width as u32,
                        rows as u32,
                        (padded_width * components) as u32,
                    );
                    match encoding {
                        14 => encoder::bc5::compress_blocks_into(
                            &RgSurface::new(pixels, w, h, row),
                            blocks,
                        ),
                        22 => encoder::bc4::compress_blocks_into(
                            &RSurface::new(pixels, w, h, row),
                            blocks,
                        ),
                        23 => encoder::bc6h::compress_blocks_into(
                            &encoder::bc6h::basic_settings(),
                            &RgbaF16Surface::new(pixels, w, h, row),
                            blocks,
                        ),
                        21 => encoder::bc7::compress_blocks_into(
                            &if opaque {
                                encoder::bc7::opaque_basic_settings()
                            } else {
                                encoder::bc7::alpha_basic_settings()
                            },
                            &RgbaSurface::new(pixels, w, h, row),
                            blocks,
                        ),
                        _ => unreachable!(),
                    }
                    Ok(())
                })?;
                result.extend(blocks);
            }
            Ok(result)
        })
        .collect::<Result<_>>()?;
    ktx2(&compressed, srgb)
}

pub fn ktx2(source: &Image, srgb: bool) -> Result<Vec<u8>> {
    let mut converted;
    let image = if matches!(source.format, 3..=5) {
        converted = source.clone();
        converted.format = 1;
        converted.levels = source
            .levels
            .iter()
            .map(|level| {
                let mut bytes = Vec::new();
                match source.format {
                    3 => {
                        for p in level.chunks_exact(2) {
                            bytes.extend([p[0], p[0], p[0], p[1]]);
                        }
                    }
                    4 => {
                        for &p in level {
                            bytes.extend([p, p, p, 255]);
                        }
                    }
                    _ => {
                        for &p in level {
                            bytes.extend([255, 255, 255, p]);
                        }
                    }
                }
                bytes
            })
            .collect();
        &converted
    } else {
        source
    };
    let (vk, vs, _, _, bs, _) = format(image.format)?;
    let dfd = descriptor(image.format, srgb)?;
    let count = image.levels.len();
    let dfd_offset = 80 + 24 * count;
    let key = b"KTXorientation\0rd\0";
    let mut kv = Vec::new();
    push32(&mut kv, key.len() as u32);
    kv.extend(key);
    kv.resize(kv.len().next_multiple_of(4), 0);
    let key_offset = dfd_offset + dfd.len();
    let mut offset = key_offset + kv.len();
    let alignment = bs.max(4);
    let mut layout = vec![(0, 0); count];
    let mut payload = Vec::new();
    for level in (0..count).rev() {
        let padding = (alignment - offset % alignment) % alignment;
        payload.resize(payload.len() + padding, 0);
        offset += padding;
        let data = &image.levels[level];
        layout[level] = (offset, data.len());
        payload.extend(data);
        offset += data.len();
    }
    let mut bytes = Vec::with_capacity(offset);
    bytes.extend(MAGIC);
    for value in [
        if srgb { vs } else { vk },
        if image.format == 19 { 2 } else { 1 },
        image.width as u32,
        image.height as u32,
        if image.flags & 8 != 0 {
            image.depth as u32
        } else {
            0
        },
        0,
        image.faces as u32,
        count as u32,
        0,
        dfd_offset as u32,
        dfd.len() as u32,
        key_offset as u32,
        kv.len() as u32,
    ] {
        push32(&mut bytes, value);
    }
    push64(&mut bytes, 0);
    push64(&mut bytes, 0);
    for (start, len) in layout {
        push64(&mut bytes, start as u64);
        push64(&mut bytes, len as u64);
        push64(&mut bytes, len as u64);
    }
    bytes.extend(dfd);
    bytes.extend(kv);
    bytes.extend(payload);
    validate(&bytes)?;
    Ok(bytes)
}
pub fn validate(bytes: &[u8]) -> Result<Value> {
    if bytes.len() < 104 || &bytes[..12] != MAGIC {
        return Err("Invalid KTX2 header".into());
    }
    let vk = u32_at(bytes, 12);
    let width = u32_at(bytes, 20) as usize;
    let height = u32_at(bytes, 24) as usize;
    let depth = u32_at(bytes, 28) as usize;
    let faces = u32_at(bytes, 36) as usize;
    let levels = u32_at(bytes, 40) as usize;
    let dfd = u32_at(bytes, 48) as usize;
    let dfd_size = u32_at(bytes, 52) as usize;
    let kv = u32_at(bytes, 56) as usize;
    let kv_size = u32_at(bytes, 60) as usize;
    if width == 0
        || height == 0
        || !matches!(faces, 1 | 6)
        || levels == 0
        || levels > 32
        || u32_at(bytes, 32) != 0
        || u32_at(bytes, 44) != 0
        || dfd != 80 + levels * 24
        || dfd_size < 44
        || dfd + dfd_size > bytes.len()
        || kv + kv_size > bytes.len()
        || u32_at(bytes, dfd) as usize != dfd_size
    {
        return Err("Unsupported KTX2 layout".into());
    }
    let (_, _, bw, bh, bs, _) = [1, 3, 4, 5, 11, 12, 13, 14, 19, 21, 22, 23]
        .into_iter()
        .filter_map(|id| format(id).ok())
        .find(|(a, b, ..)| vk == *a || vk == *b)
        .ok_or("Unknown Vulkan texture format")?;
    let mut spans = Vec::new();
    for level in 0..levels {
        let at = 80 + level * 24;
        let offset = u64_at(bytes, at) as usize;
        let size = u64_at(bytes, at + 8) as usize;
        let raw = u64_at(bytes, at + 16) as usize;
        let expected = (width >> level)
            .max(1)
            .div_ceil(bw)
            .checked_mul((height >> level).max(1).div_ceil(bh))
            .and_then(|value| value.checked_mul((depth >> level).max(1)))
            .and_then(|value| value.checked_mul(faces))
            .and_then(|value| value.checked_mul(bs))
            .ok_or("KTX2 dimensions overflow the mip size")?;
        if size != expected
            || raw != size
            || offset < kv + kv_size
            || offset.checked_add(size).is_none_or(|end| end > bytes.len())
            || offset % bs.max(4) != 0
        {
            return Err(format!("Invalid KTX2 mip {level}").into());
        }
        spans.push((offset, offset + size));
    }
    spans.sort();
    if spans.windows(2).any(|s| s[0].1 > s[1].0) {
        return Err("Overlapping KTX2 mips".into());
    }
    Ok(
        json!({"vk_format":vk,"width":width,"height":height,"depth":depth,"faces":faces,"levels":levels}),
    )
}
