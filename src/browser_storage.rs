extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};
use spin::Mutex;

const STORAGE_PATH: &str = "/CONFIG/BROWSER.STORAGE";
const MAX_STORAGE_ENTRIES: usize = 128;
const MAX_STORAGE_ORIGIN: usize = 192;
const MAX_STORAGE_KEY: usize = 64;
const MAX_STORAGE_VALUE: usize = 256;

#[derive(Clone)]
struct StorageEntry {
    origin: String,
    key: String,
    value: String,
}

struct BrowserStorage {
    loaded: bool,
    entries: Vec<StorageEntry>,
}

static LOCAL_STORAGE: Mutex<BrowserStorage> = Mutex::new(BrowserStorage {
    loaded: false,
    entries: Vec::new(),
});

pub fn local_get(origin: &str, key: &str) -> Option<String> {
    let mut storage = LOCAL_STORAGE.lock();
    storage.ensure_loaded();
    storage
        .entries
        .iter()
        .find(|entry| entry.origin == origin && entry.key == key)
        .map(|entry| entry.value.clone())
}

pub fn local_set(origin: &str, key: &str, value: &str) -> bool {
    if !valid_storage_field(origin, MAX_STORAGE_ORIGIN)
        || !valid_storage_field(key, MAX_STORAGE_KEY)
        || !valid_storage_value(value, MAX_STORAGE_VALUE)
    {
        return false;
    }
    let mut storage = LOCAL_STORAGE.lock();
    storage.ensure_loaded();
    let changed = upsert_entry(
        &mut storage.entries,
        StorageEntry {
            origin: String::from(origin),
            key: String::from(key),
            value: String::from(value),
        },
    );
    if changed {
        storage.save();
    }
    changed
}

pub fn local_remove(origin: &str, key: &str) -> bool {
    let mut storage = LOCAL_STORAGE.lock();
    storage.ensure_loaded();
    let Some(pos) = storage
        .entries
        .iter()
        .position(|entry| entry.origin == origin && entry.key == key)
    else {
        return false;
    };
    storage.entries.remove(pos);
    storage.save();
    true
}

pub fn local_clear(origin: &str) -> usize {
    let mut storage = LOCAL_STORAGE.lock();
    storage.ensure_loaded();
    let before = storage.entries.len();
    storage.entries.retain(|entry| entry.origin != origin);
    let removed = before.saturating_sub(storage.entries.len());
    if removed > 0 {
        storage.save();
    }
    removed
}

pub fn summary_line() -> String {
    let mut storage = LOCAL_STORAGE.lock();
    storage.ensure_loaded();
    format!("{} localStorage entry(s)", storage.entries.len())
}

pub fn lines() -> Vec<String> {
    let mut storage = LOCAL_STORAGE.lock();
    storage.ensure_loaded();
    let mut out = vec![
        String::from("Browser web storage"),
        String::from(""),
        format!("localStorage: {} entry(s)", storage.entries.len()),
        String::from("Storage: /CONFIG/BROWSER.STORAGE"),
        String::from("sessionStorage is per Browser document and is not persisted."),
        String::from(""),
    ];
    if storage.entries.is_empty() {
        out.push(String::from("No localStorage entries stored."));
        return out;
    }
    out.push(String::from("Stored localStorage keys"));
    for entry in storage.entries.iter().take(MAX_STORAGE_ENTRIES) {
        out.push(format!(
            "{}  {}  {} byte(s)",
            entry.origin,
            entry.key,
            entry.value.len()
        ));
    }
    out
}

pub fn storage_debug_for_test(origin: &str) -> Vec<String> {
    let _ = local_set(origin, "phase60", "stored");
    let first = local_get(origin, "phase60").unwrap_or_else(|| String::from("-"));
    let _ = local_set(origin, "phase60", "updated");
    let second = local_get(origin, "phase60").unwrap_or_else(|| String::from("-"));
    let removed = local_remove(origin, "phase60");
    let after = local_get(origin, "phase60").unwrap_or_else(|| String::from("-"));
    vec![
        format!("local_first={}", first),
        format!("local_second={}", second),
        format!("local_removed={}", removed),
        format!("local_after={}", after),
    ]
}

impl BrowserStorage {
    fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        self.entries.clear();
        let Some(bytes) = crate::config_store::read(STORAGE_PATH) else {
            return;
        };
        let Ok(text) = core::str::from_utf8(&bytes) else {
            crate::config_store::recover_corrupt(
                STORAGE_PATH,
                "/CONFIG/BROWSER.STORAGE.BAD",
                &bytes,
            );
            return;
        };
        for line in text.lines() {
            if let Some(entry) = parse_storage_line(line) {
                upsert_entry(&mut self.entries, entry);
            }
            if self.entries.len() >= MAX_STORAGE_ENTRIES {
                break;
            }
        }
    }

    fn save(&self) {
        let mut out = String::new();
        for entry in self.entries.iter().take(MAX_STORAGE_ENTRIES) {
            out.push_str("local|");
            out.push_str(&entry.origin);
            out.push('|');
            out.push_str(&entry.key);
            out.push('|');
            out.push_str(&entry.value);
            out.push('\n');
        }
        let _ = crate::config_store::safe_write(STORAGE_PATH, out.as_bytes());
    }
}

fn parse_storage_line(line: &str) -> Option<StorageEntry> {
    let mut parts = line.split('|');
    if parts.next()? != "local" {
        return None;
    }
    let origin = parts.next()?;
    let key = parts.next()?;
    let value = parts.next()?;
    if parts.next().is_some()
        || !valid_storage_field(origin, MAX_STORAGE_ORIGIN)
        || !valid_storage_field(key, MAX_STORAGE_KEY)
        || !valid_storage_value(value, MAX_STORAGE_VALUE)
    {
        return None;
    }
    Some(StorageEntry {
        origin: String::from(origin),
        key: String::from(key),
        value: String::from(value),
    })
}

fn upsert_entry(entries: &mut Vec<StorageEntry>, entry: StorageEntry) -> bool {
    if let Some(existing) = entries
        .iter_mut()
        .find(|existing| existing.origin == entry.origin && existing.key == entry.key)
    {
        *existing = entry;
        return true;
    }
    if entries.len() >= MAX_STORAGE_ENTRIES {
        entries.remove(0);
    }
    entries.push(entry);
    true
}

fn valid_storage_field(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b >= 0x20 && b < 0x7f && !matches!(b, b'\r' | b'\n' | b'|'))
}

fn valid_storage_value(value: &str, max: usize) -> bool {
    value.len() <= max
        && value
            .bytes()
            .all(|b| b >= 0x20 && b < 0x7f && !matches!(b, b'\r' | b'\n' | b'|'))
}
