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
}

impl Pairing {
    pub fn base_url(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("http://[{}]:{}", self.host, self.port)
        } else {
            format!("http://{}:{}", self.host, self.port)
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Pairing é sempre serializável")
    }

    pub fn from_json(raw: &str) -> Result<Self> {
        let p: Pairing = serde_json::from_str(raw.trim())
            .map_err(|_| SpofError::Validation("Código de pareamento inválido".into()))?;
        if p.host.trim().is_empty() || p.token.trim().is_empty() || p.port == 0 {
            return Err(SpofError::Validation("Código de pareamento incompleto".into()));
        }
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_validation() {
        let p = Pairing { host: "192.168.0.5".into(), port: 7378, token: "abc".into(), name: "PC".into() };
        assert_eq!(Pairing::from_json(&p.to_json()).unwrap(), p);
        assert_eq!(p.base_url(), "http://192.168.0.5:7378");
        assert!(Pairing::from_json("lixo").is_err());
        assert!(Pairing::from_json(r#"{"host":"","port":1,"token":"x"}"#).is_err());
        let v6 = Pairing { host: "fe80::1".into(), ..p };
        assert_eq!(v6.base_url(), "http://[fe80::1]:7378");
    }
}
