#![allow(dead_code)]

use core::sync::atomic::{AtomicU8, Ordering};

use libcool::{fs, gui, io, prelude::*};

pub const WIDTH: usize = 420;
pub const HEIGHT: usize = 252;
pub const PIXELS: usize = WIDTH * HEIGHT;
pub const MAX_TEXT: usize = 8192;
pub const LIST_BYTES: usize = 4096;

const BG: u32 = 0x0003_0712;
const PANEL: u32 = 0x0005_1324;
const PANEL_ALT: u32 = 0x0008_1c30;
const BORDER: u32 = 0x0018_4c78;
const CYAN: u32 = 0x0066_eaff;
const GREEN: u32 = 0x0088_ffcc;
const YELLOW: u32 = 0x00ff_d866;
const RED: u32 = 0x00ff_8866;
const WHITE: u32 = 0x00e8_fff8;
const MUTED: u32 = 0x0070_9ab4;

const HEADER_H: i32 = 78;
const FOOTER_Y: i32 = HEIGHT as i32 - 18;
const TEXT_Y: i32 = 92;
const LINE_H: i32 = 12;
const PAD: i32 = 16;
const BUTTON_Y: i32 = 48;
const BUTTON_H: i32 = 22;

static SMOKE_MODE: AtomicU8 = AtomicU8::new(0);

pub fn run_editor(
    args: Args,
    pixels: &mut [u32; PIXELS],
    text: &mut [u8; MAX_TEXT],
    title: &'static str,
    default_path: &'static [u8],
    label: &'static [u8],
    seed: &'static [u8],
) -> ! {
    set_smoke_mode(args);
    let path = editor_path_arg(args, default_path);
    let _ = fs::create_dir(b"/documents");

    let mut len = fs::read_file(path, text).unwrap_or(0).min(text.len());
    let mut cursor = len;
    let mut scroll = 0usize;
    let mut status = if len == 0 {
        b"new document" as &[u8]
    } else {
        b"loaded" as &[u8]
    };

    let mut window = open_or_exit(title, label);
    log(label, b"window opened");

    if smoke_mode() {
        len = 0;
        cursor = 0;
        scroll = 0;
        if insert_bytes(text, &mut len, &mut cursor, seed) {
            status = b"smoke text inserted";
        }
        match fs::write_file(path, &text[..len]) {
            Ok(()) => log_path(label, b"saved", path),
            Err(_) => log_path(label, b"save failed", path),
        }
        ensure_cursor_visible(text, len, cursor, &mut scroll);
        draw_editor(pixels, title, path, text, len, cursor, scroll, status);
        let _ = window.present(pixels);
        sleep_ms(250);
        let _ = window.close();
        exit(0);
    }

    ensure_cursor_visible(text, len, cursor, &mut scroll);
    draw_editor(pixels, title, path, text, len, cursor, scroll, status);
    let _ = window.present(pixels);

    loop {
        let mut changed = false;
        loop {
            match window.poll_event() {
                Ok(Some(gui::Event::Close)) => {
                    let _ = fs::write_file(path, &text[..len]);
                    let _ = window.close();
                    exit(0);
                }
                Ok(Some(gui::Event::KeyChar {
                    bytes,
                    len: key_len,
                })) => {
                    changed |= handle_editor_key(text, &mut len, &mut cursor, &bytes, key_len);
                }
                Ok(Some(gui::Event::MouseDown { button, x, y })) => {
                    if button == 4 {
                        scroll = scroll.saturating_sub(3);
                    } else if button == 5 {
                        let max = visual_line_count(text, len).saturating_sub(editor_rows());
                        scroll = (scroll + 3).min(max);
                    } else if hit(x, y, 18, BUTTON_Y, 72, BUTTON_H) {
                        match fs::write_file(path, &text[..len]) {
                            Ok(()) => status = b"saved",
                            Err(_) => status = b"save failed",
                        }
                    } else if hit(x, y, 100, BUTTON_Y, 92, BUTTON_H) {
                        cursor = cursor_from_click(text, len, scroll, x, y);
                    }
                }
                Ok(Some(gui::Event::Resize { .. })) => {}
                Ok(None) => break,
                Err(_) => {
                    log(label, b"event error");
                    let _ = window.close();
                    exit(1);
                }
            }
        }

        if changed {
            match fs::write_file(path, &text[..len]) {
                Ok(()) => status = b"saved",
                Err(_) => status = b"save failed",
            }
        }
        ensure_cursor_visible(text, len, cursor, &mut scroll);
        draw_editor(pixels, title, path, text, len, cursor, scroll, status);
        let _ = window.present(pixels);
        sleep_ms(25);
    }
}

pub fn run_trash(args: Args, pixels: &mut [u32; PIXELS], listing: &mut [u8; LIST_BYTES]) -> ! {
    let _ = fs::create_dir(b"/Trash");
    if smoke_arg(args) {
        let _ = fs::write_file(b"/Trash/SMOKE.TXT", b"trash smoke\n");
    }

    let mut window = open_or_exit("Trash Bin", b"trash");
    log(b"trash", b"window opened");
    let mut len = refresh_trash(listing);
    log(b"trash", b"listed");
    let mut status = b"ready" as &[u8];
    let mut scroll = 0usize;

    if smoke_arg(args) {
        empty_trash(listing, len);
        len = refresh_trash(listing);
        draw_trash(pixels, listing, len, scroll, b"empty");
        let _ = window.present(pixels);
        log(b"trash", b"empty ok");
        sleep_ms(250);
        let _ = window.close();
        exit(0);
    }

    draw_trash(pixels, listing, len, scroll, status);
    let _ = window.present(pixels);

    loop {
        let mut redraw = false;
        loop {
            match window.poll_event() {
                Ok(Some(gui::Event::Close)) => {
                    let _ = window.close();
                    exit(0);
                }
                Ok(Some(gui::Event::KeyChar {
                    bytes,
                    len: key_len,
                })) => {
                    let key = key_code(&bytes, key_len);
                    if key == Some(b'e' as u32) || key == Some(b'E' as u32) {
                        empty_trash(listing, len);
                        len = refresh_trash(listing);
                        scroll = 0;
                        status = b"emptied";
                        redraw = true;
                    } else if key == Some(b'r' as u32) || key == Some(b'R' as u32) {
                        len = refresh_trash(listing);
                        status = b"refreshed";
                        redraw = true;
                    }
                }
                Ok(Some(gui::Event::MouseDown { button, x, y })) => {
                    if button == 4 {
                        scroll = scroll.saturating_sub(3);
                        redraw = true;
                    } else if button == 5 {
                        scroll += 3;
                        redraw = true;
                    } else if hit(x, y, 18, BUTTON_Y, 88, BUTTON_H) {
                        empty_trash(listing, len);
                        len = refresh_trash(listing);
                        scroll = 0;
                        status = b"emptied";
                        redraw = true;
                    } else if hit(x, y, 116, BUTTON_Y, 82, BUTTON_H) {
                        len = refresh_trash(listing);
                        status = b"refreshed";
                        redraw = true;
                    }
                }
                Ok(Some(gui::Event::Resize { .. })) => redraw = true,
                Ok(None) => break,
                Err(_) => {
                    log(b"trash", b"event error");
                    let _ = window.close();
                    exit(1);
                }
            }
        }

        let max = list_line_count(&listing[..len]).saturating_sub(list_rows());
        scroll = scroll.min(max);
        if redraw {
            draw_trash(pixels, listing, len, scroll, status);
            let _ = window.present(pixels);
        }
        sleep_ms(40);
    }
}

pub fn run_screenshot(args: Args, pixels: &mut [u32; PIXELS]) -> ! {
    set_smoke_mode(args);
    let _ = fs::create_dir(b"/Pictures");
    let mut window = open_or_exit("Screenshot", b"screenshot");
    log(b"screenshot", b"window opened");
    draw_screenshot(pixels, b"ready");
    let _ = window.present(pixels);

    if smoke_mode() {
        queue_screenshot(b"/Pictures/SMOKE.PPM");
        sleep_ms(150);
        let _ = window.close();
        exit(0);
    }

    loop {
        loop {
            match window.poll_event() {
                Ok(Some(gui::Event::Close)) => {
                    let _ = window.close();
                    exit(0);
                }
                Ok(Some(gui::Event::KeyChar { bytes, len })) => {
                    let key = key_code(&bytes, len);
                    if key == Some(b's' as u32)
                        || key == Some(b'S' as u32)
                        || key == Some(b'\n' as u32)
                    {
                        queue_screenshot(b"/Pictures/USERSS.PPM");
                        let _ = window.close();
                        exit(0);
                    }
                }
                Ok(Some(gui::Event::MouseDown { x, y, .. })) => {
                    if hit(x, y, 18, BUTTON_Y, 92, BUTTON_H) {
                        queue_screenshot(b"/Pictures/USERSS.PPM");
                        let _ = window.close();
                        exit(0);
                    }
                }
                Ok(Some(gui::Event::Resize { .. })) => {
                    draw_screenshot(pixels, b"ready");
                    let _ = window.present(pixels);
                }
                Ok(None) => break,
                Err(_) => {
                    log(b"screenshot", b"event error");
                    let _ = window.close();
                    exit(1);
                }
            }
        }
        sleep_ms(40);
    }
}

fn open_or_exit(title: &'static str, label: &'static [u8]) -> gui::Window {
    match gui::Window::open(title.as_bytes(), WIDTH as u16, HEIGHT as u16) {
        Ok(window) => {
            sleep_ms(100);
            window
        }
        Err(_) => {
            log(label, b"open failed");
            exit(1);
        }
    }
}

fn draw_editor(
    pixels: &mut [u32; PIXELS],
    title: &str,
    path: &[u8],
    text: &[u8],
    len: usize,
    cursor: usize,
    scroll: usize,
    status: &[u8],
) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    draw_base(&mut canvas, title, path);
    button(&mut canvas, 18, BUTTON_Y, 72, BUTTON_H, "Save", GREEN);
    button(&mut canvas, 100, BUTTON_Y, 92, BUTTON_H, "Cursor", CYAN);
    panel(
        &mut canvas,
        PAD,
        TEXT_Y,
        WIDTH as i32 - PAD * 2,
        FOOTER_Y - TEXT_Y - 4,
    );

    let rows = editor_rows();
    for row in 0..rows {
        if let Some((start, end)) = visual_line_bounds(text, len, scroll + row) {
            draw_bytes(
                &mut canvas,
                PAD + 6,
                TEXT_Y + 8 + row as i32 * LINE_H,
                &text[start..end],
                WHITE,
            );
        }
    }

    let (cursor_row, cursor_col) = cursor_visual(text, len, cursor);
    if cursor_row >= scroll && cursor_row < scroll + rows {
        let x = PAD + 6 + cursor_col.min(editor_cols()) as i32 * 8;
        let y = TEXT_Y + 8 + (cursor_row - scroll) as i32 * LINE_H;
        canvas.rect(x, y, 2, 10, YELLOW);
    }

    footer(&mut canvas, status);
}

fn draw_trash(
    pixels: &mut [u32; PIXELS],
    listing: &[u8],
    len: usize,
    scroll: usize,
    status: &[u8],
) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    draw_base(&mut canvas, "Trash Bin", b"/Trash");
    button(&mut canvas, 18, BUTTON_Y, 88, BUTTON_H, "Empty", RED);
    button(&mut canvas, 116, BUTTON_Y, 82, BUTTON_H, "Refresh", CYAN);
    panel(
        &mut canvas,
        PAD,
        TEXT_Y,
        WIDTH as i32 - PAD * 2,
        FOOTER_Y - TEXT_Y - 4,
    );

    let mut visible = 0usize;
    let mut line_no = 0usize;
    let mut start = 0usize;
    let data = &listing[..len];
    for idx in 0..=data.len() {
        if idx < data.len() && data[idx] != b'\n' {
            continue;
        }
        if line_no >= scroll && visible < list_rows() {
            draw_listing_line(
                &mut canvas,
                PAD + 6,
                TEXT_Y + 8 + visible as i32 * LINE_H,
                &data[start..idx],
            );
            visible += 1;
        }
        line_no += 1;
        start = idx.saturating_add(1);
    }
    if len == 0 {
        canvas.text(PAD + 6, TEXT_Y + 8, "Trash is empty", MUTED);
    }
    footer(&mut canvas, status);
}

fn draw_screenshot(pixels: &mut [u32; PIXELS], status: &[u8]) {
    let mut canvas = gui::Canvas::new(&mut pixels[..], WIDTH, HEIGHT);
    draw_base(&mut canvas, "Screenshot", b"/Pictures/USERSS.PPM");
    button(&mut canvas, 18, BUTTON_Y, 92, BUTTON_H, "Capture", GREEN);
    panel(&mut canvas, PAD, TEXT_Y, WIDTH as i32 - PAD * 2, 92);
    canvas.text(PAD + 8, TEXT_Y + 14, "Press Enter or click Capture.", WHITE);
    canvas.text(
        PAD + 8,
        TEXT_Y + 34,
        "The utility closes before capture",
        MUTED,
    );
    canvas.text(
        PAD + 8,
        TEXT_Y + 46,
        "so the previous focused window is saved.",
        MUTED,
    );
    footer(&mut canvas, status);
}

fn draw_base(canvas: &mut gui::Canvas<'_>, title: &str, path: &[u8]) {
    canvas.clear(BG);
    canvas.rect(0, 0, WIDTH as i32, HEADER_H, PANEL_ALT);
    canvas.rect(0, HEADER_H - 1, WIDTH as i32, 1, BORDER);
    canvas.rect(0, FOOTER_Y - 1, WIDTH as i32, 1, BORDER);
    canvas.text(PAD, 14, title, WHITE);
    draw_bytes(canvas, PAD, 30, path, MUTED);
}

fn button(canvas: &mut gui::Canvas<'_>, x: i32, y: i32, w: i32, h: i32, label: &str, accent: u32) {
    canvas.rect(x, y, w, h, PANEL);
    canvas.border(x, y, w, h, accent);
    canvas.text(x + 8, y + 7, label, WHITE);
}

fn panel(canvas: &mut gui::Canvas<'_>, x: i32, y: i32, w: i32, h: i32) {
    canvas.rect(x, y, w, h, PANEL);
    canvas.border(x, y, w, h, BORDER);
}

fn footer(canvas: &mut gui::Canvas<'_>, status: &[u8]) {
    draw_bytes(canvas, PAD, FOOTER_Y + 4, status, MUTED);
}

fn draw_listing_line(canvas: &mut gui::Canvas<'_>, x: i32, y: i32, line: &[u8]) {
    if line.len() < 3 {
        return;
    }
    let prefix = if line[0] == b'D' { "[DIR] " } else { "      " };
    canvas.text(x, y, prefix, MUTED);
    let name_start = 2usize;
    let name_end = line[name_start..]
        .iter()
        .position(|&b| b == b'\t')
        .map(|idx| name_start + idx)
        .unwrap_or(line.len());
    draw_bytes(canvas, x + 48, y, &line[name_start..name_end], WHITE);
}

fn draw_bytes(canvas: &mut gui::Canvas<'_>, mut x: i32, y: i32, bytes: &[u8], color: u32) {
    let max_chars = ((WIDTH as i32 - x - PAD) / 8).max(0) as usize;
    for (idx, &byte) in bytes.iter().enumerate() {
        if idx >= max_chars {
            break;
        }
        let ch = if byte == b'\t' {
            ' '
        } else if (0x20..=0x7e).contains(&byte) {
            byte as char
        } else {
            '?'
        };
        canvas.char(x, y, ch, color);
        x += 8;
    }
}

fn handle_editor_key(
    text: &mut [u8],
    len: &mut usize,
    cursor: &mut usize,
    bytes: &[u8; 4],
    key_len: usize,
) -> bool {
    match key_code(bytes, key_len) {
        Some(8) | Some(127) => delete_prev(text, len, cursor),
        Some(0xf702) => {
            *cursor = cursor.saturating_sub(1);
            false
        }
        Some(0xf703) => {
            *cursor = (*cursor + 1).min(*len);
            false
        }
        Some(0xf704) => {
            *cursor = line_start(text, *cursor);
            false
        }
        Some(0xf705) => {
            *cursor = line_end(text, *len, *cursor);
            false
        }
        Some(code) if code == b'\n' as u32 => insert_bytes(text, len, cursor, b"\n"),
        Some(code) if (0x20..=0x7e).contains(&code) => {
            insert_bytes(text, len, cursor, &[code as u8])
        }
        _ => false,
    }
}

fn insert_bytes(text: &mut [u8], len: &mut usize, cursor: &mut usize, input: &[u8]) -> bool {
    if input.is_empty() || *len + input.len() > text.len() {
        return false;
    }
    *cursor = (*cursor).min(*len);
    for idx in (*cursor..*len).rev() {
        text[idx + input.len()] = text[idx];
    }
    text[*cursor..*cursor + input.len()].copy_from_slice(input);
    *cursor += input.len();
    *len += input.len();
    true
}

fn delete_prev(text: &mut [u8], len: &mut usize, cursor: &mut usize) -> bool {
    if *cursor == 0 || *len == 0 {
        return false;
    }
    let start = cursor.saturating_sub(1);
    for idx in start..len.saturating_sub(1) {
        text[idx] = text[idx + 1];
    }
    *cursor = start;
    *len -= 1;
    true
}

fn refresh_trash(listing: &mut [u8; LIST_BYTES]) -> usize {
    fs::list_dir(b"/Trash", listing)
        .unwrap_or(0)
        .min(listing.len())
}

fn empty_trash(listing: &[u8; LIST_BYTES], len: usize) {
    let mut start = 0usize;
    let data = &listing[..len];
    for idx in 0..=data.len() {
        if idx < data.len() && data[idx] != b'\n' {
            continue;
        }
        delete_listing_line(&data[start..idx]);
        start = idx.saturating_add(1);
    }
}

fn delete_listing_line(line: &[u8]) {
    if line.len() < 3 {
        return;
    }
    let name_start = 2usize;
    let Some(name_len) = line[name_start..].iter().position(|&b| b == b'\t') else {
        return;
    };
    let name = &line[name_start..name_start + name_len];
    if name.is_empty() || name.len() > 112 {
        return;
    }
    let mut path = [0u8; 128];
    let prefix = b"/Trash/";
    path[..prefix.len()].copy_from_slice(prefix);
    path[prefix.len()..prefix.len() + name.len()].copy_from_slice(name);
    let _ = fs::delete_tree(&path[..prefix.len() + name.len()]);
}

fn queue_screenshot(path: &[u8]) {
    match fs::screenshot(path) {
        Ok(()) => {
            io::write_stdout(b"screenshot: queued ");
            io::write_stdout(path);
            io::write_stdout(b"\n");
        }
        Err(_) => log(b"screenshot", b"queue failed"),
    }
}

fn hit(x: u16, y: u16, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    let x = x as i32;
    let y = y as i32;
    x >= bx && x < bx + bw && y >= by && y < by + bh
}

fn key_code(bytes: &[u8; 4], len: usize) -> Option<u32> {
    match len {
        1 => Some(bytes[0] as u32),
        3 if bytes[0] == 0xef && bytes[1] >= 0x9c && bytes[2] >= 0x80 => {
            Some(0xf000 + ((bytes[1] as u32 - 0x80) << 6) + (bytes[2] as u32 - 0x80))
        }
        _ => None,
    }
}

fn editor_cols() -> usize {
    ((WIDTH as i32 - PAD * 2 - 12) / 8) as usize
}

fn editor_rows() -> usize {
    ((FOOTER_Y - TEXT_Y - 18) / LINE_H) as usize
}

fn list_rows() -> usize {
    editor_rows()
}

fn visual_line_count(text: &[u8], len: usize) -> usize {
    let mut count = 0usize;
    while visual_line_bounds(text, len, count).is_some() {
        count += 1;
    }
    count.max(1)
}

fn visual_line_bounds(text: &[u8], len: usize, target: usize) -> Option<(usize, usize)> {
    let cols = editor_cols().max(1);
    let mut line = 0usize;
    let mut start = 0usize;
    let mut col = 0usize;
    let mut idx = 0usize;
    while idx < len {
        if text[idx] == b'\n' {
            if line == target {
                return Some((start, idx));
            }
            line += 1;
            idx += 1;
            start = idx;
            col = 0;
            continue;
        }
        if col >= cols {
            if line == target {
                return Some((start, idx));
            }
            line += 1;
            start = idx;
            col = 0;
        }
        idx += 1;
        col += 1;
    }
    if line == target {
        Some((start, len))
    } else {
        None
    }
}

fn cursor_visual(text: &[u8], len: usize, cursor: usize) -> (usize, usize) {
    let cursor = cursor.min(len);
    let mut line = 0usize;
    while let Some((start, end)) = visual_line_bounds(text, len, line) {
        if cursor >= start && cursor <= end {
            return (line, cursor.saturating_sub(start));
        }
        line += 1;
    }
    (0, 0)
}

fn ensure_cursor_visible(text: &[u8], len: usize, cursor: usize, scroll: &mut usize) {
    let (row, _) = cursor_visual(text, len, cursor);
    let rows = editor_rows().max(1);
    if row < *scroll {
        *scroll = row;
    } else if row >= *scroll + rows {
        *scroll = row.saturating_sub(rows - 1);
    }
}

fn cursor_from_click(text: &[u8], len: usize, scroll: usize, x: u16, y: u16) -> usize {
    let x = x as i32;
    let y = y as i32;
    if x < PAD + 6 || y < TEXT_Y + 8 {
        return len;
    }
    let row = scroll + ((y - TEXT_Y - 8) / LINE_H).max(0) as usize;
    let col = ((x - PAD - 6) / 8).max(0) as usize;
    if let Some((start, end)) = visual_line_bounds(text, len, row) {
        (start + col).min(end)
    } else {
        len
    }
}

fn line_start(text: &[u8], cursor: usize) -> usize {
    let mut idx = cursor.min(text.len());
    while idx > 0 && text[idx - 1] != b'\n' {
        idx -= 1;
    }
    idx
}

fn line_end(text: &[u8], len: usize, cursor: usize) -> usize {
    let mut idx = cursor.min(len);
    while idx < len && text[idx] != b'\n' {
        idx += 1;
    }
    idx
}

fn list_line_count(data: &[u8]) -> usize {
    data.iter().filter(|&&b| b == b'\n').count().max(1)
}

fn set_smoke_mode(args: Args) {
    SMOKE_MODE.store(smoke_arg(args) as u8, Ordering::Relaxed);
}

fn smoke_mode() -> bool {
    SMOKE_MODE.load(Ordering::Relaxed) != 0
}

fn arg_is(args: Args, index: usize, expected: &[u8]) -> bool {
    matches!(args.get(index), Some(arg) if arg == expected)
}

fn arg_any(args: Args, expected: &[u8]) -> bool {
    let mut idx = 1usize;
    while idx < args.len() {
        if arg_is(args, idx, expected) {
            return true;
        }
        idx += 1;
    }
    false
}

fn smoke_arg(args: Args) -> bool {
    arg_any(args, b"smoke") || arg_any(args, b"s")
}

fn editor_path_arg(args: Args, default_path: &[u8]) -> &[u8] {
    let mut idx = 1usize;
    while idx < args.len() {
        if let Some(arg) = args.get(idx) {
            if arg.first() == Some(&b'/') {
                return arg;
            }
        }
        idx += 1;
    }
    default_path
}

fn log(label: &[u8], message: &[u8]) {
    io::write_stdout(label);
    io::write_stdout(b": ");
    io::write_stdout(message);
    io::write_stdout(b"\n");
}

fn log_path(label: &[u8], message: &[u8], path: &[u8]) {
    io::write_stdout(label);
    io::write_stdout(b": ");
    io::write_stdout(message);
    io::write_stdout(b" ");
    io::write_stdout(path);
    io::write_stdout(b"\n");
}
