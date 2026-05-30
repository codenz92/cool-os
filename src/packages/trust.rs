fn package_signature(path: &str) -> Result<PackageSignature, &'static str> {
    let bytes = crate::vfs::vfs_kernel_read_file(path).ok_or("package is unsigned")?;
    let text = core::str::from_utf8(&bytes).map_err(|_| "package signature is not UTF-8")?;
    let signature = PackageSignature {
        key: String::from(manifest_value(text, "key").unwrap_or("")),
        algorithm: String::from(manifest_value(text, "algorithm").unwrap_or("")),
        package_id: String::from(manifest_value(text, "package_id").unwrap_or("")),
        package_version: String::from(manifest_value(text, "package_version").unwrap_or("")),
        issued_epoch: manifest_value(text, "issued_epoch")
            .and_then(parse_u64)
            .unwrap_or(0),
        manifest_sha256: String::from(manifest_value(text, "manifest_sha256").unwrap_or("")),
        signature: String::from(manifest_value(text, "signature").unwrap_or("")),
    };
    if signature.key.is_empty()
        || signature.algorithm.is_empty()
        || signature.package_id.is_empty()
        || signature.package_version.is_empty()
        || signature.issued_epoch == 0
        || signature.manifest_sha256.is_empty()
        || signature.signature.is_empty()
    {
        return Err("package signature incomplete");
    }
    Ok(signature)
}

fn write_signature(
    path: &str,
    archive: &ArchiveManifest,
    key_id: &str,
) -> Result<(), &'static str> {
    let seed = signing_seed(key_id).ok_or("signing key unavailable")?;
    let manifest_sha256 = crate::update_crypto::digest_hex(archive.trust_manifest.as_bytes());
    let signature = crate::update_crypto::hex(&crate::update_crypto::ed25519_sign(
        seed,
        archive.trust_manifest.as_bytes(),
    ));
    let mut out = String::from("coolOS package signature\n");
    out.push_str("key=");
    out.push_str(key_id);
    out.push_str("\nalgorithm=");
    out.push_str(TRUST_ALGORITHM);
    out.push_str("\npackage_id=");
    out.push_str(&archive.id);
    out.push_str("\npackage_version=");
    out.push_str(&archive.version);
    out.push_str("\nissued_epoch=");
    out.push_str(&format!("{}", CURRENT_EPOCH));
    out.push_str("\nmanifest_sha256=");
    out.push_str(&manifest_sha256);
    out.push_str("\nsignature=");
    out.push_str(&signature);
    out.push('\n');
    safe_write(&signature_path(path), out.as_bytes())
}

fn trusted_keys() -> Vec<TrustedKey> {
    ensure_trust_keys_file();
    let Some(bytes) = crate::vfs::vfs_kernel_read_file(TRUST_KEYS_PATH) else {
        return built_in_trusted_keys();
    };
    let Ok(text) = core::str::from_utf8(&bytes) else {
        return built_in_trusted_keys();
    };
    let mut keys = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || !line.starts_with("key=") {
            continue;
        }
        let Some(id) = token_value(line, "key=") else {
            continue;
        };
        let Some(public_hex) = token_value(line, "public=") else {
            continue;
        };
        let Some(public_key) = crate::update_crypto::hex_to_32(public_hex) else {
            continue;
        };
        keys.push(TrustedKey {
            id: String::from(id),
            algorithm: String::from(token_value(line, "algorithm=").unwrap_or(TRUST_ALGORITHM)),
            status: String::from(token_value(line, "status=").unwrap_or("untrusted")),
            public_key,
            not_before: token_value(line, "not_before=")
                .and_then(parse_u64)
                .unwrap_or(1),
            not_after: token_value(line, "not_after=")
                .and_then(parse_u64)
                .unwrap_or(MAX_EPOCH),
            generation: token_value(line, "generation=")
                .and_then(parse_u64)
                .unwrap_or(0),
        });
    }
    if keys.is_empty() {
        built_in_trusted_keys()
    } else {
        keys
    }
}

fn find_trusted_key(id: &str) -> Option<TrustedKey> {
    trusted_keys().into_iter().find(|key| key.id == id)
}

fn built_in_trusted_keys() -> Vec<TrustedKey> {
    alloc::vec![
        TrustedKey {
            id: String::from("phase69-pkg-a"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: PKG_A_PUBLIC,
            not_before: 1,
            not_after: MAX_EPOCH,
            generation: 1,
        },
        TrustedKey {
            id: String::from("phase69-pkg-b"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: PKG_B_PUBLIC,
            not_before: 1,
            not_after: MAX_EPOCH,
            generation: 2,
        },
        TrustedKey {
            id: String::from("phase69-pkg-revoked"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("revoked"),
            public_key: PKG_REVOKED_PUBLIC,
            not_before: 1,
            not_after: MAX_EPOCH,
            generation: 0,
        },
        TrustedKey {
            id: String::from("phase69-pkg-expired"),
            algorithm: String::from(TRUST_ALGORITHM),
            status: String::from("trusted"),
            public_key: PKG_EXPIRED_PUBLIC,
            not_before: 1,
            not_after: 1,
            generation: 0,
        },
    ]
}

fn signing_seed(key_id: &str) -> Option<&'static [u8; 32]> {
    match key_id {
        "phase69-pkg-a" => Some(&PKG_A_SEED),
        "phase69-pkg-b" => Some(&PKG_B_SEED),
        "phase69-pkg-revoked" => Some(&PKG_REVOKED_SEED),
        "phase69-pkg-expired" => Some(&PKG_EXPIRED_SEED),
        "phase69-pkg-unknown" => Some(&PKG_B_SEED),
        _ => None,
    }
}
