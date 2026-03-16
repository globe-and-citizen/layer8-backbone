use std::sync::Arc;
use arc_swap::ArcSwap;
use boring::pkey::PKey;
use boring::x509::X509;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use pingora::utils::tls::CertKey;
use serde::Deserialize;
use x509_parser::parse_x509_certificate;
use x509_parser::pem::parse_x509_pem;
use crate::deserializer;

pub fn extract_x509_pem(pem: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (_, pem) = parse_x509_pem(pem.as_bytes())?;

    // Parse the certificate
    let (_, cert) = parse_x509_certificate(&pem.contents)?;

    // Extract public key bytes
    let spki = cert.public_key().clone();
    let pubkey_bytes = spki.subject_public_key.data;

    Ok(pubkey_bytes.to_vec())
}

#[derive(Debug, Deserialize, Clone)]
pub struct TLSPathConfig {
    #[serde(deserialize_with = "deserializer::string_to_bool")]
    pub enable_tls: bool,
    #[serde(default)]
    pub path_to_ca_cert: String,
    #[serde(default)]
    pub path_to_cert: String,
    #[serde(default)]
    pub path_to_key: String,
}

pub struct TLSConfig {
    pub ca_cert: X509,
    pub cert_key: ArcSwap<CertKey>,
}

impl TLSConfig {
    pub fn load(conf: &TLSPathConfig) -> Result<TLSConfig, String> {
        let ca_pem = std::fs::read(&conf.path_to_ca_cert)
            .map_err(|e| format!("Failed to read CA certificate: {}", e))?;

        let cert_pem = std::fs::read(&conf.path_to_cert)
            .map_err(|e| format!("Failed to read certificate: {}", e))?;

        let key_pem = std::fs::read(&conf.path_to_key)
            .map_err(|e| format!("Failed to read key: {}", e))?;

        let ca_cert = X509::from_pem(&ca_pem)
            .map_err(|e| format!("Invalid CA certificate: {}", e))?;

        let cert = X509::stack_from_pem(&cert_pem)
            .map_err(|e| format!("Invalid certificate: {}", e))?;

        let key = PKey::private_key_from_pem(&key_pem)
            .map_err(|e| format!("Invalid private key: {}", e))?;

        // The certificate chain to present in mTLS connections to upstream
        let cert_key = CertKey::new(cert, key);

        Ok(TLSConfig{
            ca_cert,
            cert_key: ArcSwap::from_pointee(cert_key),
        })
    }

    pub fn reload(&self, path: &TLSPathConfig) -> Result<(), String> {
        let cert_pem = std::fs::read(&path.path_to_cert)
            .map_err(|e| format!("Failed to reload certificate: {}", e))?;
        let key_pem = std::fs::read(&path.path_to_key)
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

pub fn watch_tls(config: Arc<TLSConfig>, path: TLSPathConfig) {
    let watch_path = path.path_to_cert.clone();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            match res {
                Ok(_event) => {
                    let _ = config.reload(&path);
                }
                Err(e) => {
                    eprintln!("watch error: {:?}", e);
                }
            }
        },
        notify::Config::default(),
    ).unwrap();

    watcher.watch(watch_path.as_ref(), RecursiveMode::NonRecursive).unwrap();
}
