use std::sync::Arc;
use arc_swap::ArcSwap;
use boring::pkey::PKey;
use boring::x509::X509;
use pingora::utils::tls::CertKey;
use serde::Deserialize;
use crate::deserializer;

#[derive(Debug, Deserialize, Clone)]
pub struct TLSConfig {
    #[serde(default, deserialize_with = "deserializer::string_to_bool")]
    pub enable_tls: bool,
    #[serde(default)]
    pub ca_path: String,
    #[serde(default)]
    pub cert_path: String,
    #[serde(default)]
    pub key_path: String,
}

pub struct TLSCredentials {
    pub ca_cert: X509,
    pub cert_key: ArcSwap<CertKey>,
}

impl TLSCredentials {
    pub fn load(conf: &TLSConfig) -> Result<TLSCredentials, String> {
        let ca_pem = std::fs::read(&conf.ca_path)
            .map_err(|e| format!("Failed to read CA certificate: {}", e))?;

        let cert_pem = std::fs::read(&conf.cert_path)
            .map_err(|e| format!("Failed to read certificate: {}", e))?;

        let key_pem = std::fs::read(&conf.key_path)
            .map_err(|e| format!("Failed to read key: {}", e))?;

        let ca_cert = X509::from_pem(&ca_pem)
            .map_err(|e| format!("Invalid CA certificate: {}", e))?;

        let cert = X509::stack_from_pem(&cert_pem)
            .map_err(|e| format!("Invalid certificate: {}", e))?;

        let key = PKey::private_key_from_pem(&key_pem)
            .map_err(|e| format!("Invalid private key: {}", e))?;

        // The certificate chain to present in mTLS connections to upstream
        let cert_key = CertKey::new(cert, key);

        Ok(TLSCredentials {
            ca_cert,
            cert_key: ArcSwap::from_pointee(cert_key),
        })
    }

    pub fn reload(&self, path: &TLSConfig) -> Result<(), String> {
        let cert_pem = std::fs::read(&path.cert_path)
            .map_err(|e| format!("Failed to reload certificate: {}", e))?;
        let key_pem = std::fs::read(&path.key_path)
            .map_err(|e| format!("Failed to reload key: {}", e))?;

        let cert = X509::stack_from_pem(&cert_pem)
            .map_err(|e| format!("Invalid certificate: {}", e))?;
        let key = PKey::private_key_from_pem(&key_pem)
            .map_err(|e| format!("Invalid private key: {}", e))?;

        let new_cert_key = CertKey::new(cert, key);

        self.cert_key.store(Arc::new(new_cert_key));

        Ok(())
    }
}

