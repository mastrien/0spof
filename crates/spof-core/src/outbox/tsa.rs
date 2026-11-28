use crate::error::{Result, SpofError};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::time::Duration;

pub const DEFAULT_TSA_URL: &str = "https://freetsa.org/tsr";

/// Builds an RFC 3161 ASN.1 DER-encoded TimeStampReq for a SHA-256 hash.
pub fn build_rfc3161_request(hash_bytes: &[u8; 32], cert_req: bool) -> Vec<u8> {
    // SHA-256 AlgorithmIdentifier with NULL parameters (15 bytes)
    // 30 0d 06 09 60 86 48 01 65 03 04 02 01 05 00
    let mut algo_id = vec![
        0x30, 0x0d, // SEQUENCE, 13 bytes
        0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
        0x01, // OID 2.16.840.1.101.3.4.2.1
        0x05, 0x00, // NULL
    ];

    // HashedMessage: OCTET STRING, 32 bytes (34 bytes total)
    let mut hashed_msg = vec![0x04, 0x20];
    hashed_msg.extend_from_slice(hash_bytes);

    // MessageImprint: SEQUENCE containing algo_id + hashed_msg (49 bytes)
    let mut imprint = vec![0x30, 0x31];
    imprint.append(&mut algo_id);
    imprint.append(&mut hashed_msg);

    // TimeStampReq body: version (INTEGER 1) + MessageImprint
    let mut req_body = vec![
        0x02, 0x01, 0x01, // INTEGER 1 (v1)
    ];
    req_body.append(&mut imprint);

    if cert_req {
        // certReq BOOLEAN TRUE (01 01 ff)
        req_body.extend_from_slice(&[0x01, 0x01, 0xff]);
    }

    // Wrap in outer SEQUENCE
    let mut der = vec![0x30, req_body.len() as u8];
    der.extend_from_slice(&req_body);
    der
}

/// Client for communicating with an RFC 3161 Timestamping Authority (TSA).
#[derive(Clone)]
pub struct TsaClient {
    tsa_url: String,
    http: reqwest::Client,
}

impl TsaClient {
    pub fn new(tsa_url: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            tsa_url: tsa_url.unwrap_or_else(|| DEFAULT_TSA_URL.to_string()),
            http,
        }
    }

    pub fn tsa_url(&self) -> &str {
        &self.tsa_url
    }

    /// Requests an RFC 3161 cryptographic timestamp for the given SHA-256 hexadecimal hash.
    pub async fn request_timestamp(&self, hash_hex: &str) -> Result<Vec<u8>> {
        let decoded = hex_to_bytes(hash_hex)
            .ok_or_else(|| SpofError::Validation("Invalid SHA-256 hex string".to_string()))?;

        if decoded.len() != 32 {
            return Err(SpofError::Validation(
                "SHA-256 hash must be exactly 32 bytes (64 hex characters)".to_string(),
            ));
        }

        let mut hash_arr = [0u8; 32];
        hash_arr.copy_from_slice(&decoded);

        let der_request = build_rfc3161_request(&hash_arr, true);

        let response = self
            .http
            .post(&self.tsa_url)
            .header("Content-Type", "application/timestamp-query")
            .header("Accept", "application/timestamp-reply")
            .body(der_request)
            .send()
            .await
            .map_err(|e| {
                SpofError::Network(format!(
                    "Failed to reach TSA server {}: {}",
                    self.tsa_url, e
                ))
            })?;

        if !response.status().is_success() {
            return Err(SpofError::Network(format!(
                "TSA server returned status: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| SpofError::Network(format!("Failed to read TSA response: {}", e)))?;

        Ok(bytes.to_vec())
    }
}

/// Extrai o horário (`genTime`) de um carimbo RFC 3161 em DER.
///
/// Não valida a assinatura: apenas localiza o primeiro `GeneralizedTime` (tag 0x18) do token,
/// que na estrutura TimeStampResp é o `genTime` do TSTInfo (certificados vêm depois e usam
/// `UTCTime`). Serve para exibir "carimbado em" ao usuário.
pub fn parse_timestamp_gen_time(token: &[u8]) -> Option<DateTime<Utc>> {
    let mut i = 0;
    while i + 2 < token.len() {
        if token[i] == 0x18 {
            let len = token[i + 1] as usize;
            if (15..=32).contains(&len) && i + 2 + len <= token.len() {
                let raw = &token[i + 2..i + 2 + len];
                let digits_ok = raw[..14].iter().all(u8::is_ascii_digit);
                if digits_ok && raw.ends_with(b"Z") {
                    let text = std::str::from_utf8(&raw[..14]).ok()?;
                    let naive = NaiveDateTime::parse_from_str(text, "%Y%m%d%H%M%S").ok()?;
                    return Some(naive.and_utc());
                }
            }
        }
        i += 1;
    }
    None
}

fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_rfc3161_request_structure() {
        let dummy_hash = [0xabu8; 32];
        let der = build_rfc3161_request(&dummy_hash, true);

        // Must start with SEQUENCE tag 0x30
        assert_eq!(der[0], 0x30);
        // Minimum length check (approx 58-60 bytes)
        assert!(der.len() >= 55);

        // Check for version 1
        assert_eq!(&der[2..5], &[0x02, 0x01, 0x01]);
    }

    #[test]
    fn test_parse_timestamp_gen_time_finds_generalized_time() {
        // Lixo + GeneralizedTime "20261128104200Z" (15 bytes) + resto.
        let mut token = vec![0x30, 0x82, 0x01, 0x00, 0x02, 0x01, 0x01];
        token.extend_from_slice(&[0x18, 0x0f]);
        token.extend_from_slice(b"20261128104200Z");
        token.extend_from_slice(&[0x17, 0x0d]);
        token.extend_from_slice(b"270101000000Z");
        let t = parse_timestamp_gen_time(&token).expect("genTime");
        assert_eq!(t.to_rfc3339(), "2026-11-28T10:42:00+00:00");
    }

    #[test]
    fn test_parse_timestamp_gen_time_accepts_fraction_and_rejects_garbage() {
        let mut token = vec![0x18, 0x13];
        token.extend_from_slice(b"20260102030405.123Z");
        assert_eq!(
            parse_timestamp_gen_time(&token).unwrap().to_rfc3339(),
            "2026-01-02T03:04:05+00:00"
        );
        assert!(parse_timestamp_gen_time(&[]).is_none());
        assert!(parse_timestamp_gen_time(&[0x30, 0x03, 0x01, 0x02, 0x03]).is_none());
        // tag 0x18 mas conteúdo não numérico
        let mut bad = vec![0x18, 0x0f];
        bad.extend_from_slice(b"abcdefghijklmnZ");
        assert!(parse_timestamp_gen_time(&bad).is_none());
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("e3b0"), Some(vec![0xe3, 0xb0]));
        assert_eq!(hex_to_bytes("invalid_odd_len"), None);
        assert_eq!(hex_to_bytes("zz"), None);
    }

    #[test]
    fn test_tsa_client_url() {
        let client = TsaClient::new(None);
        assert_eq!(client.tsa_url(), DEFAULT_TSA_URL);

        let custom = TsaClient::new(Some("https://custom.tsa.org/".to_string()));
        assert_eq!(custom.tsa_url(), "https://custom.tsa.org/");
    }

    #[tokio::test]
    async fn test_request_timestamp_invalid_hash_validation() {
        let client = TsaClient::new(None);
        let res = client.request_timestamp("curto").await;
        assert!(res.is_err());
    }
}
