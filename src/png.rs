extern crate alloc;

use alloc::{vec, vec::Vec};

const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const COLOR_RGB: u8 = 2;
const COLOR_RGBA: u8 = 6;

#[derive(Clone)]
pub struct PngImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

pub fn decode_rgb8(data: &[u8], max_pixels: usize) -> Result<PngImage, &'static str> {
    if data.len() < PNG_SIG.len() || &data[..PNG_SIG.len()] != PNG_SIG {
        return Err("bad PNG signature");
    }

    let mut pos = PNG_SIG.len();
    let mut width = 0usize;
    let mut height = 0usize;
    let mut color_type = 0u8;
    let mut seen_ihdr = false;
    let mut idat = Vec::new();

    while pos + 12 <= data.len() {
        let len = read_be_u32(data, pos).ok_or("bad PNG chunk")? as usize;
        let kind = data.get(pos + 4..pos + 8).ok_or("bad PNG chunk")?;
        let chunk_start = pos + 8;
        let chunk_end = chunk_start.checked_add(len).ok_or("bad PNG chunk")?;
        let crc_end = chunk_end.checked_add(4).ok_or("bad PNG chunk")?;
        if crc_end > data.len() {
            return Err("truncated PNG chunk");
        }
        let chunk = &data[chunk_start..chunk_end];
        match kind {
            b"IHDR" => {
                if len != 13 {
                    return Err("bad PNG IHDR");
                }
                width = read_be_u32(chunk, 0).ok_or("bad PNG width")? as usize;
                height = read_be_u32(chunk, 4).ok_or("bad PNG height")? as usize;
                let bit_depth = chunk[8];
                color_type = chunk[9];
                if width == 0 || height == 0 || width.saturating_mul(height) > max_pixels {
                    return Err("PNG dimensions too large");
                }
                if bit_depth != 8 || !(color_type == COLOR_RGB || color_type == COLOR_RGBA) {
                    return Err("unsupported PNG color format");
                }
                if chunk[10] != 0 || chunk[11] != 0 || chunk[12] != 0 {
                    return Err("unsupported PNG compression");
                }
                seen_ihdr = true;
            }
            b"IDAT" => {
                if !seen_ihdr {
                    return Err("PNG IDAT before IHDR");
                }
                idat.extend_from_slice(chunk);
            }
            b"IEND" => break,
            _ => {}
        }
        pos = crc_end;
    }

    if !seen_ihdr || idat.is_empty() {
        return Err("incomplete PNG");
    }

    let bpp = if color_type == COLOR_RGBA { 4 } else { 3 };
    let row_bytes = width.checked_mul(bpp).ok_or("PNG row too wide")?;
    let expected = height
        .checked_mul(row_bytes.checked_add(1).ok_or("PNG row too wide")?)
        .ok_or("PNG image too large")?;
    let inflated = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&idat, expected)
        .map_err(|_| "PNG inflate failed")?;
    if inflated.len() < expected {
        return Err("truncated PNG data");
    }

    let mut prev = vec![0u8; row_bytes];
    let mut cur = vec![0u8; row_bytes];
    let mut pixels = Vec::with_capacity(width * height);
    let mut src = 0usize;
    for _ in 0..height {
        let filter = inflated[src];
        src += 1;
        cur.copy_from_slice(&inflated[src..src + row_bytes]);
        src += row_bytes;
        unfilter_row(filter, bpp, &prev, &mut cur)?;
        for px in cur.chunks_exact(bpp) {
            let r = px[0] as u32;
            let g = px[1] as u32;
            let b = px[2] as u32;
            let (r, g, b) = if bpp == 4 {
                blend_over_white(r, g, b, px[3] as u32)
            } else {
                (r, g, b)
            };
            pixels.push((r << 16) | (g << 8) | b);
        }
        prev.copy_from_slice(&cur);
    }

    Ok(PngImage {
        width,
        height,
        pixels,
    })
}

fn unfilter_row(filter: u8, bpp: usize, prev: &[u8], cur: &mut [u8]) -> Result<(), &'static str> {
    match filter {
        0 => {}
        1 => {
            for i in 0..cur.len() {
                let left = if i >= bpp { cur[i - bpp] } else { 0 };
                cur[i] = cur[i].wrapping_add(left);
            }
        }
        2 => {
            for i in 0..cur.len() {
                cur[i] = cur[i].wrapping_add(prev[i]);
            }
        }
        3 => {
            for i in 0..cur.len() {
                let left = if i >= bpp { cur[i - bpp] } else { 0 };
                let up = prev[i];
                cur[i] = cur[i].wrapping_add(((left as u16 + up as u16) / 2) as u8);
            }
        }
        4 => {
            for i in 0..cur.len() {
                let left = if i >= bpp { cur[i - bpp] } else { 0 };
                let up = prev[i];
                let up_left = if i >= bpp { prev[i - bpp] } else { 0 };
                cur[i] = cur[i].wrapping_add(paeth(left, up, up_left));
            }
        }
        _ => return Err("unsupported PNG filter"),
    }
    Ok(())
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

fn blend_over_white(r: u32, g: u32, b: u32, a: u32) -> (u32, u32, u32) {
    let inv = 255 - a;
    (
        (r * a + 255 * inv) / 255,
        (g * a + 255 * inv) / 255,
        (b * a + 255 * inv) / 255,
    )
}

fn read_be_u32(data: &[u8], pos: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *data.get(pos)?,
        *data.get(pos + 1)?,
        *data.get(pos + 2)?,
        *data.get(pos + 3)?,
    ]))
}
