use crate::config::{Certificate, TlsCipherSuite, TlsClock, TlsVerifier};
use crate::der_certificate::{DecodedCertificate, Time, ECDSA_SHA256, ECDSA_SHA384, ED25519};
#[cfg(feature = "rsa")]
use crate::der_certificate::{RSA_PKCS1_SHA256, RSA_PKCS1_SHA384, RSA_PKCS1_SHA512};
use crate::extensions::extension_data::signature_algorithms::SignatureScheme;
use crate::handshake::{
    certificate::{
        Certificate as OwnedCertificate, CertificateEntryRef, CertificateRef as ServerCertificate,
    },
    certificate_verify::CertificateVerifyRef,
};
use crate::parse_buffer::ParseError;
use crate::TlsError;
use const_oid::ObjectIdentifier;
use core::marker::PhantomData;
use der::Decode;
use der::Tagged;
use digest::Digest;
use heapless::{String, Vec};

const HOSTNAME_MAXLEN: usize = 64;
const SAN_DNS_MAX: usize = 16;
const SAN_IP_MAX: usize = 8;
const COMMON_NAME_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.3");
const SUBJECT_ALT_NAME_OID: &[u8] = &[0x55, 0x1d, 0x11];

pub struct CertificateChain<'a> {
    prev: Option<&'a CertificateEntryRef<'a>>,
    chain: &'a ServerCertificate<'a>,
    idx: isize,
}

impl<'a> CertificateChain<'a> {
    pub fn new(ca: &'a CertificateEntryRef, chain: &'a ServerCertificate<'a>) -> Self {
        let mut idx = chain.entries.len() as isize - 1;
        while idx >= 0 && certificate_has_same_spki(ca, &chain.entries[idx as usize]) {
            idx -= 1;
        }
        Self {
            prev: Some(ca),
            chain,
            idx,
        }
    }
}

impl<'a> Iterator for CertificateChain<'a> {
    type Item = (&'a CertificateEntryRef<'a>, &'a CertificateEntryRef<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < 0 {
            return None;
        }

        let cur = &self.chain.entries[self.idx as usize];
        let out = (self.prev.unwrap(), cur);

        self.prev = Some(cur);
        self.idx -= 1;

        Some(out)
    }
}

fn certificate_has_same_spki(left: &CertificateEntryRef, right: &CertificateEntryRef) -> bool {
    let (CertificateEntryRef::X509(left), CertificateEntryRef::X509(right)) = (left, right) else {
        return false;
    };
    let Ok(left) = DecodedCertificate::from_der(left) else {
        return false;
    };
    let Ok(right) = DecodedCertificate::from_der(right) else {
        return false;
    };
    let left_spki = &left.tbs_certificate.subject_public_key_info;
    let right_spki = &right.tbs_certificate.subject_public_key_info;
    left_spki.algorithm.oid == right_spki.algorithm.oid
        && left_spki.public_key.as_bytes() == right_spki.public_key.as_bytes()
}

struct CertificateIdentity {
    common_name: Option<heapless::String<HOSTNAME_MAXLEN>>,
    dns_names: Vec<heapless::String<HOSTNAME_MAXLEN>, SAN_DNS_MAX>,
    ip4_addrs: Vec<[u8; 4], SAN_IP_MAX>,
    saw_dns_name: bool,
    dns_name_matched: bool,
    ip4_addr_matched: bool,
}

impl CertificateIdentity {
    fn new() -> Self {
        Self {
            common_name: None,
            dns_names: Vec::new(),
            ip4_addrs: Vec::new(),
            saw_dns_name: false,
            dns_name_matched: false,
            ip4_addr_matched: false,
        }
    }
}

pub struct CertVerifier<CipherSuite, Clock, const CERT_SIZE: usize>
where
    Clock: TlsClock,
    CipherSuite: TlsCipherSuite,
{
    host: Option<heapless::String<64>>,
    certificate_transcript: Option<CipherSuite::Hash>,
    certificate: Option<OwnedCertificate<CERT_SIZE>>,
    _clock: PhantomData<Clock>,
}

impl<Cs, C, const CERT_SIZE: usize> Default for CertVerifier<Cs, C, CERT_SIZE>
where
    C: TlsClock,
    Cs: TlsCipherSuite,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<CipherSuite, Clock, const CERT_SIZE: usize> CertVerifier<CipherSuite, Clock, CERT_SIZE>
where
    Clock: TlsClock,
    CipherSuite: TlsCipherSuite,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            host: None,
            certificate_transcript: None,
            certificate: None,
            _clock: PhantomData,
        }
    }
}

impl<CipherSuite, Clock, const CERT_SIZE: usize> TlsVerifier<CipherSuite>
    for CertVerifier<CipherSuite, Clock, CERT_SIZE>
where
    CipherSuite: TlsCipherSuite,
    Clock: TlsClock,
{
    fn set_hostname_verification(&mut self, hostname: &str) -> Result<(), TlsError> {
        self.host.replace(
            heapless::String::try_from(hostname).map_err(|_| TlsError::InsufficientSpace)?,
        );
        Ok(())
    }

    fn verify_certificate(
        &mut self,
        transcript: &CipherSuite::Hash,
        ca: &Option<Certificate>,
        cert: ServerCertificate,
    ) -> Result<(), TlsError> {
        let ca = if let Some(ca) = ca {
            ca
        } else {
            error!("Verifying a certificate chain without ca is not implemented");
            return Err(TlsError::Unimplemented);
        };

        let mut identity = CertificateIdentity::new();
        let verify_host = self.host.as_ref().map(|host| host.as_str());
        for (p, q) in CertificateChain::new(&ca.into(), &cert) {
            identity = verify_certificate(p, q, Clock::now(), verify_host)?;
        }
        if let Some(host) = self.host.as_ref() {
            if !hostname_matches_identity(host, &identity) {
                error!(
                    "Hostname ({:?}) does not match SAN/CN ({:?})",
                    self.host, identity.common_name
                );
                return Err(TlsError::InvalidCertificateRequest);
            }
        }

        self.certificate.replace(cert.try_into()?);
        self.certificate_transcript.replace(transcript.clone());
        Ok(())
    }

    fn verify_signature(&mut self, verify: CertificateVerifyRef) -> Result<(), TlsError> {
        let handshake_hash = unwrap!(self.certificate_transcript.take());
        let ctx_str = b"TLS 1.3, server CertificateVerify\x00";
        let mut msg: Vec<u8, 146> = Vec::new();
        msg.resize(64, 0x20).map_err(|_| TlsError::EncodeError)?;
        msg.extend_from_slice(ctx_str)
            .map_err(|_| TlsError::EncodeError)?;
        msg.extend_from_slice(&handshake_hash.finalize())
            .map_err(|_| TlsError::EncodeError)?;

        let certificate = unwrap!(self.certificate.as_ref()).try_into()?;
        verify_signature(&msg[..], &certificate, &verify)?;
        Ok(())
    }
}

pub fn hostname_matches_for_test(
    host: &str,
    common_name: Option<&str>,
    dns_names: &[&str],
    ip4_addrs: &[[u8; 4]],
) -> bool {
    let mut identity = CertificateIdentity::new();
    if let Some(common_name) = common_name {
        if let Ok(name) = dns_name_from_ascii(common_name.as_bytes()) {
            identity.common_name = Some(name);
        }
    }
    for dns_name in dns_names {
        identity.saw_dns_name = true;
        if let Ok(name) = dns_name_from_ascii(dns_name.as_bytes()) {
            if dns_name_matches(&name, host) {
                identity.dns_name_matched = true;
            }
            let _ = identity.dns_names.push(name);
        }
    }
    let host_ip = parse_ipv4_host(host);
    for ip in ip4_addrs {
        if host_ip == Some(*ip) {
            identity.ip4_addr_matched = true;
        }
        let _ = identity.ip4_addrs.push(*ip);
    }
    hostname_matches_identity(host, &identity)
}

fn verify_signature(
    message: &[u8],
    certificate: &ServerCertificate,
    verify: &CertificateVerifyRef,
) -> Result<(), TlsError> {
    let verified;

    let certificate =
        if let Some(CertificateEntryRef::X509(certificate)) = certificate.entries.first() {
            certificate
        } else {
            return Err(TlsError::DecodeError);
        };

    let certificate =
        DecodedCertificate::from_der(certificate).map_err(|_| TlsError::DecodeError)?;

    let public_key = certificate
        .tbs_certificate
        .subject_public_key_info
        .public_key
        .as_bytes()
        .ok_or(TlsError::DecodeError)?;

    match verify.signature_scheme {
        SignatureScheme::EcdsaSecp256r1Sha256 => {
            use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
            let verifying_key =
                VerifyingKey::from_sec1_bytes(public_key).map_err(|_| TlsError::DecodeError)?;
            let signature =
                Signature::from_der(&verify.signature).map_err(|_| TlsError::DecodeError)?;
            verified = verifying_key.verify(message, &signature).is_ok();
        }
        SignatureScheme::EcdsaSecp384r1Sha384 => {
            use p384::ecdsa::{signature::Verifier, Signature, VerifyingKey};
            let verifying_key =
                VerifyingKey::from_sec1_bytes(public_key).map_err(|_| TlsError::DecodeError)?;
            let signature =
                Signature::from_der(&verify.signature).map_err(|_| TlsError::DecodeError)?;
            verified = verifying_key.verify(message, &signature).is_ok();
        }
        SignatureScheme::Ed25519 => {
            use ed25519_dalek::{Signature, Verifier, VerifyingKey};
            let verifying_key: VerifyingKey =
                VerifyingKey::from_bytes(public_key.try_into().unwrap())
                    .map_err(|_| TlsError::DecodeError)?;
            let signature =
                Signature::try_from(verify.signature).map_err(|_| TlsError::DecodeError)?;
            verified = verifying_key.verify(message, &signature).is_ok();
        }
        #[cfg(feature = "rsa")]
        SignatureScheme::RsaPssRsaeSha256 => {
            use rsa::{
                pkcs1::DecodeRsaPublicKey,
                pss::{Signature, VerifyingKey},
                signature::Verifier,
                RsaPublicKey,
            };
            use sha2::Sha256;

            let der_pubkey = RsaPublicKey::from_pkcs1_der(public_key).unwrap();
            let verifying_key = VerifyingKey::<Sha256>::from(der_pubkey);

            let signature =
                Signature::try_from(verify.signature).map_err(|_| TlsError::DecodeError)?;
            verified = verifying_key.verify(message, &signature).is_ok();
        }
        #[cfg(feature = "rsa")]
        SignatureScheme::RsaPssRsaeSha384 => {
            use rsa::{
                pkcs1::DecodeRsaPublicKey,
                pss::{Signature, VerifyingKey},
                signature::Verifier,
                RsaPublicKey,
            };
            use sha2::Sha384;

            let der_pubkey =
                RsaPublicKey::from_pkcs1_der(public_key).map_err(|_| TlsError::DecodeError)?;
            let verifying_key = VerifyingKey::<Sha384>::from(der_pubkey);

            let signature =
                Signature::try_from(verify.signature).map_err(|_| TlsError::DecodeError)?;
            verified = verifying_key.verify(message, &signature).is_ok();
        }
        #[cfg(feature = "rsa")]
        SignatureScheme::RsaPssRsaeSha512 => {
            use rsa::{
                pkcs1::DecodeRsaPublicKey,
                pss::{Signature, VerifyingKey},
                signature::Verifier,
                RsaPublicKey,
            };
            use sha2::Sha512;

            let der_pubkey =
                RsaPublicKey::from_pkcs1_der(public_key).map_err(|_| TlsError::DecodeError)?;
            let verifying_key = VerifyingKey::<Sha512>::from(der_pubkey);

            let signature =
                Signature::try_from(verify.signature).map_err(|_| TlsError::DecodeError)?;
            verified = verifying_key.verify(message, &signature).is_ok();
        }
        _ => {
            error!("InvalidSignatureScheme: {:?}", verify.signature_scheme);
            return Err(TlsError::InvalidSignatureScheme);
        }
    }

    if !verified {
        return Err(TlsError::InvalidSignature);
    }
    Ok(())
}

fn get_certificate_tlv_bytes<'a>(input: &[u8]) -> der::Result<&[u8]> {
    use der::{Decode, Reader, SliceReader};

    let mut reader = SliceReader::new(input)?;
    let top_header = der::Header::decode(&mut reader)?;
    top_header.tag().assert_eq(der::Tag::Sequence)?;

    let header = der::Header::peek(&mut reader)?;
    header.tag().assert_eq(der::Tag::Sequence)?;

    // Should we read the remaining two fields and call reader.finish() just be certain here?
    reader.tlv_bytes()
}

fn get_cert_time(time: Time) -> u64 {
    match time {
        Time::UtcTime(utc_time) => utc_time.to_unix_duration().as_secs(),
        Time::GeneralTime(generalized_time) => generalized_time.to_unix_duration().as_secs(),
    }
}

fn verify_certificate(
    verifier: &CertificateEntryRef,
    certificate: &CertificateEntryRef,
    now: Option<u64>,
    verify_host: Option<&str>,
) -> Result<CertificateIdentity, TlsError> {
    let mut verified = false;
    let mut identity = CertificateIdentity::new();

    let ca_certificate = if let CertificateEntryRef::X509(verifier) = verifier {
        DecodedCertificate::from_der(verifier).map_err(|_| TlsError::DecodeError)?
    } else {
        return Err(TlsError::DecodeError);
    };

    if let CertificateEntryRef::X509(certificate) = certificate {
        let parsed_certificate =
            DecodedCertificate::from_der(certificate).map_err(|_| TlsError::DecodeError)?;

        let ca_public_key = ca_certificate
            .tbs_certificate
            .subject_public_key_info
            .public_key
            .as_bytes()
            .ok_or(TlsError::DecodeError)?;

        for elems in parsed_certificate.tbs_certificate.subject.iter() {
            let attrs = elems
                .get(0)
                .ok_or(TlsError::ParseError(ParseError::InvalidData))?;
            if attrs.oid == COMMON_NAME_OID {
                let mut v: Vec<u8, HOSTNAME_MAXLEN> = Vec::new();
                v.extend_from_slice(attrs.value.value())
                    .map_err(|_| TlsError::ParseError(ParseError::InvalidData))?;
                identity.common_name = String::from_utf8(v).ok();
                debug!("CommonName: {:?}", identity.common_name);
            }
        }

        collect_subject_alt_names(
            parsed_certificate.tbs_certificate.extensions.as_ref(),
            &mut identity,
            verify_host,
        )?;

        if let Some(now) = now {
            if get_cert_time(parsed_certificate.tbs_certificate.validity.not_before) > now
                || get_cert_time(parsed_certificate.tbs_certificate.validity.not_after) < now
            {
                return Err(TlsError::InvalidCertificate);
            }
            debug!("Epoch is {} and certificate is valid!", now)
        }

        let certificate_data =
            get_certificate_tlv_bytes(certificate).map_err(|_| TlsError::DecodeError)?;

        match parsed_certificate.signature_algorithm {
            ECDSA_SHA256 => {
                use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
                let verifying_key = VerifyingKey::from_sec1_bytes(ca_public_key)
                    .map_err(|_| TlsError::DecodeError)?;

                let signature = Signature::from_der(
                    parsed_certificate
                        .signature
                        .as_bytes()
                        .ok_or(TlsError::ParseError(ParseError::InvalidData))?,
                )
                .map_err(|_| TlsError::ParseError(ParseError::InvalidData))?;

                verified = verifying_key.verify(&certificate_data, &signature).is_ok();
            }
            ECDSA_SHA384 => {
                use p384::ecdsa::{signature::Verifier, Signature, VerifyingKey};
                let verifying_key = VerifyingKey::from_sec1_bytes(ca_public_key)
                    .map_err(|_| TlsError::DecodeError)?;

                let signature = Signature::from_der(
                    parsed_certificate
                        .signature
                        .as_bytes()
                        .ok_or(TlsError::ParseError(ParseError::InvalidData))?,
                )
                .map_err(|_| TlsError::ParseError(ParseError::InvalidData))?;

                verified = verifying_key.verify(&certificate_data, &signature).is_ok();
            }
            ED25519 => {
                use ed25519_dalek::{Signature, Verifier, VerifyingKey};
                let verifying_key: VerifyingKey =
                    VerifyingKey::from_bytes(ca_public_key.try_into().unwrap())
                        .map_err(|_| TlsError::DecodeError)?;

                let signature = Signature::try_from(
                    parsed_certificate
                        .signature
                        .as_bytes()
                        .ok_or(TlsError::ParseError(ParseError::InvalidData))?,
                )
                .map_err(|_| TlsError::ParseError(ParseError::InvalidData))?;

                verified = verifying_key.verify(certificate_data, &signature).is_ok();
            }
            #[cfg(feature = "rsa")]
            a if a == RSA_PKCS1_SHA256 => {
                use rsa::{
                    pkcs1::DecodeRsaPublicKey,
                    pkcs1v15::{Signature, VerifyingKey},
                    signature::Verifier,
                };
                use sha2::Sha256;

                let verifying_key =
                    VerifyingKey::<Sha256>::from_pkcs1_der(ca_public_key).map_err(|e| {
                        error!("VerifyingKey: {}", e);
                        TlsError::DecodeError
                    })?;

                let signature = Signature::try_from(
                    parsed_certificate
                        .signature
                        .as_bytes()
                        .ok_or(TlsError::ParseError(ParseError::InvalidData))?,
                )
                .map_err(|e| {
                    error!("Signature: {}", e);
                    TlsError::ParseError(ParseError::InvalidData)
                })?;

                verified = verifying_key.verify(certificate_data, &signature).is_ok();
            }
            #[cfg(feature = "rsa")]
            a if a == RSA_PKCS1_SHA384 => {
                use rsa::{
                    pkcs1::DecodeRsaPublicKey,
                    pkcs1v15::{Signature, VerifyingKey},
                    signature::Verifier,
                };
                use sha2::Sha384;

                let verifying_key = VerifyingKey::<Sha384>::from_pkcs1_der(ca_public_key)
                    .map_err(|_| TlsError::DecodeError)?;

                let signature = Signature::try_from(
                    parsed_certificate
                        .signature
                        .as_bytes()
                        .ok_or(TlsError::ParseError(ParseError::InvalidData))?,
                )
                .map_err(|_| TlsError::ParseError(ParseError::InvalidData))?;

                verified = verifying_key.verify(certificate_data, &signature).is_ok();
            }
            #[cfg(feature = "rsa")]
            a if a == RSA_PKCS1_SHA512 => {
                use rsa::{
                    pkcs1::DecodeRsaPublicKey,
                    pkcs1v15::{Signature, VerifyingKey},
                    signature::Verifier,
                };
                use sha2::Sha512;

                let verifying_key = VerifyingKey::<Sha512>::from_pkcs1_der(ca_public_key)
                    .map_err(|_| TlsError::DecodeError)?;

                let signature = Signature::try_from(
                    parsed_certificate
                        .signature
                        .as_bytes()
                        .ok_or(TlsError::ParseError(ParseError::InvalidData))?,
                )
                .map_err(|_| TlsError::ParseError(ParseError::InvalidData))?;

                verified = verifying_key.verify(certificate_data, &signature).is_ok();
            }
            _ => {
                error!(
                    "Unsupported signature alg: {:?}",
                    parsed_certificate.signature_algorithm
                );
                return Err(TlsError::InvalidSignatureScheme);
            }
        }
    }

    if !verified {
        return Err(TlsError::InvalidCertificate);
    }

    Ok(identity)
}

fn collect_subject_alt_names(
    extensions: Option<&der::AnyRef<'_>>,
    identity: &mut CertificateIdentity,
    verify_host: Option<&str>,
) -> Result<(), TlsError> {
    let Some(extensions) = extensions else {
        return Ok(());
    };
    if extensions.tag() != der::Tag::Sequence {
        return Ok(());
    }

    let mut pos = 0usize;
    let bytes = extensions.value();
    while pos < bytes.len() {
        let (ext, next) = der_value_at(bytes, pos, 0x30)?;
        pos = next;
        let mut ext_pos = 0usize;
        let (oid, next) = der_value_at(ext, ext_pos, 0x06)?;
        ext_pos = next;
        if ext.get(ext_pos) == Some(&0x01) {
            let (_, next) = der_value_at(ext, ext_pos, 0x01)?;
            ext_pos = next;
        }
        let (value, _) = der_value_at(ext, ext_pos, 0x04)?;
        if oid == SUBJECT_ALT_NAME_OID {
            collect_general_names(value, identity, verify_host)?;
        }
    }
    Ok(())
}

fn collect_general_names(
    extn_value: &[u8],
    identity: &mut CertificateIdentity,
    verify_host: Option<&str>,
) -> Result<(), TlsError> {
    let (names, _) = der_value_at(extn_value, 0, 0x30)?;
    let verify_ip = verify_host.and_then(parse_ipv4_host);
    let mut pos = 0usize;
    while pos < names.len() {
        let tag = *names
            .get(pos)
            .ok_or(TlsError::ParseError(ParseError::InvalidData))?;
        let (len, header_len) = der_len_at(names, pos + 1)?;
        let start = pos + 1 + header_len;
        let end = start
            .checked_add(len)
            .ok_or(TlsError::ParseError(ParseError::InvalidData))?;
        if end > names.len() {
            return Err(TlsError::ParseError(ParseError::InvalidData));
        }
        let value = &names[start..end];
        match tag {
            0x82 => {
                identity.saw_dns_name = true;
                if let Ok(name) = dns_name_from_ascii(value) {
                    if let Some(host) = verify_host {
                        if dns_name_matches(&name, host) {
                            identity.dns_name_matched = true;
                        }
                    }
                    let _ = identity.dns_names.push(name);
                }
            }
            0x87 if value.len() == 4 => {
                let addr = [value[0], value[1], value[2], value[3]];
                if verify_ip == Some(addr) {
                    identity.ip4_addr_matched = true;
                }
                let _ = identity.ip4_addrs.push(addr);
            }
            _ => {}
        }
        pos = end;
    }
    Ok(())
}

fn der_value_at(bytes: &[u8], pos: usize, expected_tag: u8) -> Result<(&[u8], usize), TlsError> {
    if bytes.get(pos) != Some(&expected_tag) {
        return Err(TlsError::ParseError(ParseError::InvalidData));
    }
    let (len, header_len) = der_len_at(bytes, pos + 1)?;
    let start = pos + 1 + header_len;
    let end = start
        .checked_add(len)
        .ok_or(TlsError::ParseError(ParseError::InvalidData))?;
    if end > bytes.len() {
        return Err(TlsError::ParseError(ParseError::InvalidData));
    }
    Ok((&bytes[start..end], end))
}

fn der_len_at(bytes: &[u8], pos: usize) -> Result<(usize, usize), TlsError> {
    let first = *bytes
        .get(pos)
        .ok_or(TlsError::ParseError(ParseError::InvalidData))?;
    if first & 0x80 == 0 {
        return Ok((first as usize, 1));
    }
    let count = (first & 0x7f) as usize;
    if count == 0 || count > core::mem::size_of::<usize>() {
        return Err(TlsError::ParseError(ParseError::InvalidData));
    }
    let mut len = 0usize;
    for idx in 0..count {
        let b = *bytes
            .get(pos + 1 + idx)
            .ok_or(TlsError::ParseError(ParseError::InvalidData))?;
        len = len
            .checked_mul(256)
            .and_then(|value| value.checked_add(b as usize))
            .ok_or(TlsError::ParseError(ParseError::InvalidData))?;
    }
    Ok((len, 1 + count))
}

fn dns_name_from_ascii(value: &[u8]) -> Result<heapless::String<HOSTNAME_MAXLEN>, TlsError> {
    let mut out: Vec<u8, HOSTNAME_MAXLEN> = Vec::new();
    for b in value {
        let b = b.to_ascii_lowercase();
        if !(b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'*')) {
            return Err(TlsError::ParseError(ParseError::InvalidData));
        }
        out.push(b)
            .map_err(|_| TlsError::ParseError(ParseError::InvalidData))?;
    }
    String::from_utf8(out).map_err(|_| TlsError::ParseError(ParseError::InvalidData))
}

fn hostname_matches_identity(host: &str, identity: &CertificateIdentity) -> bool {
    if let Some(ip) = parse_ipv4_host(host) {
        if identity.ip4_addr_matched {
            return true;
        }
        return identity.ip4_addrs.iter().any(|candidate| *candidate == ip);
    }

    if identity.dns_name_matched {
        return true;
    }

    if !identity.dns_names.is_empty() {
        return identity
            .dns_names
            .iter()
            .any(|name| dns_name_matches(name, host));
    }

    if identity.saw_dns_name {
        return false;
    }

    identity
        .common_name
        .as_ref()
        .map(|name| dns_name_matches(name, host))
        .unwrap_or(false)
}

fn dns_name_matches(pattern: &str, host: &str) -> bool {
    let pattern = trim_trailing_dot(pattern);
    let host = trim_trailing_dot(host);
    if let Some(suffix) = pattern.strip_prefix("*.") {
        if !suffix.contains('.') {
            return false;
        }
        let Some(rest) = strip_suffix_ascii_ignore_case(host, suffix) else {
            return false;
        };
        return rest.ends_with('.') && rest[..rest.len() - 1].find('.').is_none();
    }
    ascii_eq_ignore_case(pattern, host)
}

fn trim_trailing_dot(value: &str) -> &str {
    value.strip_suffix('.').unwrap_or(value)
}

fn ascii_eq_ignore_case(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .all(|(l, r)| l.to_ascii_lowercase() == r.to_ascii_lowercase())
}

fn strip_suffix_ascii_ignore_case<'a>(value: &'a str, suffix: &str) -> Option<&'a str> {
    if suffix.len() > value.len() {
        return None;
    }
    let start = value.len() - suffix.len();
    ascii_eq_ignore_case(&value[start..], suffix).then_some(&value[..start])
}

fn parse_ipv4_host(host: &str) -> Option<[u8; 4]> {
    let host = trim_trailing_dot(host);
    let mut out = [0u8; 4];
    let mut count = 0usize;
    for part in host.split('.') {
        if count >= 4 || part.is_empty() || part.len() > 3 {
            return None;
        }
        let mut value = 0u16;
        for b in part.bytes() {
            if !b.is_ascii_digit() {
                return None;
            }
            value = value.checked_mul(10)?.checked_add((b - b'0') as u16)?;
            if value > 255 {
                return None;
            }
        }
        out[count] = value as u8;
        count += 1;
    }
    (count == 4).then_some(out)
}
