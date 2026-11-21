use crate::error::{Result, SpofError};
use serde::{Deserialize, Serialize};

/// Conteúdo do QR code: onde o servidor está e o token secreto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Pairing {
    pub host: String,
    pub port: u16,
    pub token: String,
    /// Nome do aparelho servidor (ex.: "DESKTOP-XYZ").
    #[serde(default)]
    pub name: String,
    /// Outros IPs do PC, tentados quando `host` não responde (VPN, máquina virtual, IP que mudou).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alt_hosts: Vec<String>,
}

impl Pairing {
    pub fn base_url(&self) -> String {
        self.base_url_for(&self.host)
    }

    pub fn base_url_for(&self, host: &str) -> String {
        if host.contains(':') && !host.starts_with('[') {
            format!("http://[{}]:{}", host, self.port)
        } else {
            format!("http://{}:{}", host, self.port)
        }
    }

    /// `host` primeiro, depois os alternativos, sem repetição.
    pub fn hosts(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for h in std::iter::once(&self.host).chain(&self.alt_hosts) {
            let h = h.trim();
            if !h.is_empty() && !out.iter().any(|x| x == h) {
                out.push(h.to_string());
            }
        }
        out
    }

    /// O mesmo pareamento com `host` como principal (o que respondeu por último).
    pub fn with_primary(&self, host: &str) -> Pairing {
        let alt_hosts = self.hosts().into_iter().filter(|h| h != host).collect();
        Pairing {
            host: host.to_string(),
            alt_hosts,
            ..self.clone()
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Pairing é sempre serializável")
    }

    pub fn from_json(raw: &str) -> Result<Self> {
        let p: Pairing = serde_json::from_str(raw.trim())
            .map_err(|_| SpofError::Validation("Código de pareamento inválido".into()))?;
        if p.host.trim().is_empty() || p.token.trim().is_empty() || p.port == 0 {
            return Err(SpofError::Validation(
                "Código de pareamento incompleto".into(),
            ));
        }
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_validation() {
        let p = Pairing {
            host: "192.168.0.5".into(),
            port: 7378,
            token: "abc".into(),
            name: "PC".into(),
            alt_hosts: vec![],
        };
        assert_eq!(Pairing::from_json(&p.to_json()).unwrap(), p);
        // QR antigo (sem alt_hosts) continua válido.
        assert!(Pairing::from_json(r#"{"host":"1.2.3.4","port":1,"token":"x"}"#).is_ok());
        assert_eq!(p.base_url(), "http://192.168.0.5:7378");
        assert!(Pairing::from_json("lixo").is_err());
        assert!(Pairing::from_json(r#"{"host":"","port":1,"token":"x"}"#).is_err());
        let v6 = Pairing {
            host: "fe80::1".into(),
            ..p
        };
        assert_eq!(v6.base_url(), "http://[fe80::1]:7378");
    }

    #[test]
    fn hosts_order_dedup_and_promotion() {
        let p = Pairing {
            host: "26.1.1.1".into(),
            port: 7378,
            token: "t".into(),
            name: "PC".into(),
            alt_hosts: vec![
                "192.168.0.5".into(),
                "26.1.1.1".into(),
                " ".into(),
                "10.0.0.2".into(),
            ],
        };
        assert_eq!(p.hosts(), ["26.1.1.1", "192.168.0.5", "10.0.0.2"]);
        let q = p.with_primary("192.168.0.5");
        assert_eq!(q.host, "192.168.0.5");
        assert_eq!(q.hosts(), ["192.168.0.5", "26.1.1.1", "10.0.0.2"]);
        assert_eq!(q.token, "t");
    }
}
