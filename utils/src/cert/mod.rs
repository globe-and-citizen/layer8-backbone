mod setup;
pub use setup::*;

use std::sync::Arc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use x509_parser::parse_x509_certificate;
use x509_parser::pem::parse_x509_pem;

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
    let watch_path = config.cert_path.clone();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            match res {
                Ok(_event) => {
                    let _ = credentials.reload(&config);
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
