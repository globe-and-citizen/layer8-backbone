mod setup;
pub use setup::*;

use std::sync::Arc;
use tracing::{error, info};
use x509_parser::parse_x509_certificate;
use x509_parser::pem::parse_x509_pem;
use blake3;

pub fn extract_x509_pem(pem: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (_, pem) = parse_x509_pem(pem.as_bytes())?;

    // Parse the certificate
    let (_, cert) = parse_x509_certificate(&pem.contents)?;

    // Extract public key bytes
    let spki = cert.public_key().clone();
    let pubkey_bytes = spki.subject_public_key.data;

    Ok(pubkey_bytes.to_vec())
}

pub fn watch_tls(credentials: Arc<TLSCredentials>, config: TLSConfig) {
    std::thread::spawn(move || {
        let mut last_hash = None;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));

            let cert = match std::fs::read(&config.cert_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let key = match std::fs::read(&config.key_path) {
                Ok(k) => k,
                Err(_) => continue,
            };

            let hash = blake3::hash(&[cert.as_slice(), key.as_slice()].concat());

            if Some(hash) != last_hash {
                if let Err(e) = credentials.reload(&config) {
                    error!("TLS reload failed: {}", e);
                } else {
                    info!("TLS reloaded");
                    last_hash = Some(hash);
                }
            }
        }
    });
}
