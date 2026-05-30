enum BrowserFetchedImage {
    Png {
        image: crate::png::PngImage,
        source_url: String,
        byte_len: usize,
        cache_hit: bool,
    },
    Placeholder {
        label: String,
        source_url: String,
        byte_len: usize,
        cache_hit: bool,
    },
}

fn fetch_image_for_browser(
    url: &str,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> Result<BrowserFetchedImage, String> {
    let resource =
        fetch_subresource_with_cache(url, BrowserResourceKind::Image, cache, stats, bypass_cache)
            .map_err(String::from)?;
    let cache_hit = resource.cache_hit;
    if !is_image_content(resource.content_type.as_deref())
        && !looks_like_image_bytes(&resource.bytes)
        && !is_known_image_path(&resource.final_url)
    {
        stats.images_failed = stats.images_failed.saturating_add(1);
        return Err(String::from("preview skipped: response is not image data"));
    }
    if !is_png_content(resource.content_type.as_deref(), &resource.final_url) {
        let label = image_metadata_label(
            &resource.bytes,
            resource.content_type.as_deref(),
            &resource.final_url,
        )
        .unwrap_or_else(|| String::from("image metadata unavailable"));
        stats.image_placeholders = stats.image_placeholders.saturating_add(1);
        return Ok(BrowserFetchedImage::Placeholder {
            label,
            source_url: resource.final_url,
            byte_len: resource.bytes.len(),
            cache_hit,
        });
    }
    match crate::png::decode_rgb8(&resource.bytes, MAX_INLINE_PNG_PIXELS) {
        Ok(image) => {
            stats.images_loaded = stats.images_loaded.saturating_add(1);
            Ok(BrowserFetchedImage::Png {
                image,
                source_url: resource.final_url,
                byte_len: resource.bytes.len(),
                cache_hit,
            })
        }
        Err(err) => {
            stats.images_failed = stats.images_failed.saturating_add(1);
            Err(format!("PNG preview unavailable: {}", err))
        }
    }
}

fn fetch_subresource_with_cache(
    url: &str,
    kind: BrowserResourceKind,
    cache: &mut BrowserSubresourceCache,
    stats: &mut BrowserSubresourceStats,
    bypass_cache: bool,
) -> Result<BrowserFetchedResource, &'static str> {
    if !bypass_cache {
        if let Some(resource) = cache.lookup(url, kind) {
            stats.note_cache(true);
            return Ok(resource);
        }
    }
    stats.note_cache(false);
    let resource = load_subresource_uncached(url, kind)?;
    cache.remember(
        url,
        kind,
        &resource.final_url,
        resource.content_type.clone(),
        &resource.bytes,
    );
    Ok(resource)
}

fn load_subresource_uncached(
    url: &str,
    kind: BrowserResourceKind,
) -> Result<BrowserFetchedResource, &'static str> {
    if let Some(path) = url.strip_prefix("file://") {
        let bytes = crate::vfs::vfs_read_file(path).ok_or("subresource file missing")?;
        if bytes.len() > MAX_BROWSER_RESOURCE_BYTES {
            return Err("subresource too large");
        }
        let content_type = browser_resource_content_type(kind, path, &bytes);
        return Ok(BrowserFetchedResource {
            final_url: file_url_for_path(path),
            content_type,
            bytes,
            cache_hit: false,
        });
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("unsupported subresource URL");
    }
    let response = crate::net::browser_get_response(url)?;
    if response.body_bytes.len() > MAX_BROWSER_RESOURCE_BYTES {
        return Err("subresource too large");
    }
    Ok(BrowserFetchedResource {
        final_url: response.final_url,
        content_type: response.content_type,
        bytes: response.body_bytes,
        cache_hit: false,
    })
}

fn browser_resource_content_type(
    kind: BrowserResourceKind,
    path: &str,
    bytes: &[u8],
) -> Option<String> {
    match kind {
        BrowserResourceKind::Stylesheet => {
            if extension_from_path(path).eq_ignore_ascii_case("css") {
                Some(String::from("text/css"))
            } else {
                Some(String::from("text/plain"))
            }
        }
        BrowserResourceKind::Image => image_content_type_for(path, bytes).map(String::from),
        BrowserResourceKind::Script => {
            if extension_from_path(path).eq_ignore_ascii_case("js") {
                Some(String::from("application/javascript"))
            } else {
                Some(String::from("text/plain"))
            }
        }
    }
}

fn inline_image_spacer(_slot: usize, url: &str) -> BrowserLine {
    BrowserLine::new(
        String::new(),
        Some(String::from(url)),
        BrowserLineKind::Image,
    )
}

fn inline_image_reserved_rows_for(width: usize, height: usize, cols: usize) -> usize {
    let max_w = cols.saturating_mul(CHAR_W).max(80);
    let (_draw_w, draw_h) = scaled_image_size(width, height, max_w, INLINE_IMAGE_MAX_H);
    (draw_h / LINE_H + 3).clamp(4, INLINE_IMAGE_RESERVED_ROWS)
}

fn scaled_image_size(image_w: usize, image_h: usize, max_w: usize, max_h: usize) -> (usize, usize) {
    if image_w == 0 || image_h == 0 || max_w == 0 || max_h == 0 {
        return (0, 0);
    }
    if image_w <= max_w && image_h <= max_h {
        let mut scale = 1usize;
        while scale < 16
            && image_w.saturating_mul(scale + 1) <= max_w
            && image_h.saturating_mul(scale + 1) <= max_h
            && image_w.saturating_mul(scale) < 320
            && image_h.saturating_mul(scale) < 220
        {
            scale += 1;
        }
        return (image_w.saturating_mul(scale), image_h.saturating_mul(scale));
    }
    let mut draw_w = image_w.min(max_w);
    let mut draw_h = image_h.saturating_mul(draw_w) / image_w;
    if draw_h > max_h {
        draw_h = max_h;
        draw_w = image_w.saturating_mul(draw_h) / image_h;
    }
    (draw_w.min(max_w), draw_h.min(max_h))
}

fn scaled_image_size_with_hint(
    image_w: usize,
    image_h: usize,
    hint: ImageHint,
    max_w: usize,
    max_h: usize,
) -> (usize, usize) {
    if image_w == 0 || image_h == 0 || max_w == 0 || max_h == 0 {
        return (0, 0);
    }
    let Some(mut draw_w) = hint.width else {
        let Some(mut draw_h) = hint.height else {
            return scaled_image_size(image_w, image_h, max_w, max_h);
        };
        draw_h = draw_h.clamp(1, max_h);
        let draw_w = image_w
            .saturating_mul(draw_h)
            .saturating_div(image_h)
            .max(1);
        return fit_box(draw_w, draw_h, max_w, max_h);
    };
    draw_w = draw_w.clamp(1, max_w);
    let draw_h = hint
        .height
        .unwrap_or_else(|| {
            image_h
                .saturating_mul(draw_w)
                .saturating_div(image_w)
                .max(1)
        })
        .clamp(1, max_h);
    fit_box(draw_w, draw_h, max_w, max_h)
}

fn fit_box(mut w: usize, mut h: usize, max_w: usize, max_h: usize) -> (usize, usize) {
    if w > max_w {
        h = h.saturating_mul(max_w).saturating_div(w).max(1);
        w = max_w;
    }
    if h > max_h {
        w = w.saturating_mul(max_h).saturating_div(h).max(1);
        h = max_h;
    }
    (w.min(max_w), h.min(max_h))
}

fn image_alt_from_line(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("[image]") {
        let label = rest.trim();
        if !label.is_empty() {
            return String::from(label);
        }
    }
    if let Some(rest) = text.strip_prefix("[image ") {
        if let Some((_, label)) = rest.split_once(']') {
            let label = label.trim();
            if !label.is_empty() {
                return String::from(label);
            }
        }
    }
    String::from("image")
}

fn image_metadata_label(bytes: &[u8], content_type: Option<&str>, url: &str) -> Option<String> {
    let kind = image_kind_label(content_type, url, bytes);
    if let Some((w, h)) = png_dimensions(bytes)
        .or_else(|| gif_dimensions(bytes))
        .or_else(|| jpeg_dimensions(bytes))
        .or_else(|| webp_dimensions(bytes))
    {
        Some(format!("{} {}x{}", kind, w, h))
    } else if kind != "image" {
        Some(String::from(kind))
    } else {
        None
    }
}

fn image_kind_label(content_type: Option<&str>, url: &str, bytes: &[u8]) -> &'static str {
    let ct = content_type
        .and_then(|value| value.split(';').next())
        .unwrap_or("")
        .trim();
    if ct.eq_ignore_ascii_case("image/png") || bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "PNG"
    } else if ct.eq_ignore_ascii_case("image/jpeg")
        || ct.eq_ignore_ascii_case("image/jpg")
        || bytes.starts_with(b"\xff\xd8")
    {
        "JPEG"
    } else if ct.eq_ignore_ascii_case("image/gif")
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
    {
        "GIF"
    } else if ct.eq_ignore_ascii_case("image/webp")
        || (bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
    {
        "WebP"
    } else {
        match extension_from_path(url) {
            "png" => "PNG",
            "jpg" => "JPEG",
            "gif" => "GIF",
            "webp" => "WebP",
            _ => "image",
        }
    }
}

fn png_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let w = read_be_u32_local(bytes, 16)? as usize;
    let h = read_be_u32_local(bytes, 20)? as usize;
    if w == 0 || h == 0 {
        None
    } else {
        Some((w, h))
    }
}

fn gif_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 10 || !(bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return None;
    }
    let w = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
    let h = u16::from_le_bytes([bytes[8], bytes[9]]) as usize;
    if w == 0 || h == 0 {
        None
    } else {
        Some((w, h))
    }
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 4 || !bytes.starts_with(b"\xff\xd8") {
        return None;
    }
    let mut pos = 2usize;
    while pos + 9 < bytes.len() && pos < 4096 {
        if bytes[pos] != 0xff {
            pos += 1;
            continue;
        }
        while pos < bytes.len() && bytes[pos] == 0xff {
            pos += 1;
        }
        let marker = *bytes.get(pos)?;
        pos += 1;
        if matches!(marker, 0xd8 | 0xd9 | 0x01) {
            continue;
        }
        let len = read_be_u16_local(bytes, pos)? as usize;
        if len < 2 || pos + len > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            let h = read_be_u16_local(bytes, pos + 3)? as usize;
            let w = read_be_u16_local(bytes, pos + 5)? as usize;
            if w > 0 && h > 0 {
                return Some((w, h));
            }
            return None;
        }
        pos += len;
    }
    None
}

fn webp_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 30 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }
    if &bytes[12..16] == b"VP8X" && bytes.len() >= 30 {
        let w = 1 + read_le_u24_local(bytes, 24)? as usize;
        let h = 1 + read_le_u24_local(bytes, 27)? as usize;
        return Some((w, h));
    }
    None
}

fn read_be_u32_local(bytes: &[u8], pos: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *bytes.get(pos)?,
        *bytes.get(pos + 1)?,
        *bytes.get(pos + 2)?,
        *bytes.get(pos + 3)?,
    ]))
}

fn read_be_u16_local(bytes: &[u8], pos: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*bytes.get(pos)?, *bytes.get(pos + 1)?]))
}

fn read_le_u24_local(bytes: &[u8], pos: usize) -> Option<u32> {
    Some(
        (*bytes.get(pos)? as u32)
            | ((*bytes.get(pos + 1)? as u32) << 8)
            | ((*bytes.get(pos + 2)? as u32) << 16),
    )
}
