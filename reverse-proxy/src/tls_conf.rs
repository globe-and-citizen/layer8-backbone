use boring::{
    pkey::{PKey},
    ssl::{SslAlert, SslRef, SslVerifyError, SslVerifyMode},
    x509::{X509StoreContext},
};
use boring::stack::Stack;
use boring::x509::store::X509StoreBuilder;
use boring::x509::X509;
use pingora::{listeners::TlsAccept, protocols::tls::TlsRef};
use serde::Deserialize;
use tracing::{error, info};
use crate::handler::common::consts::LogTypes;

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    #[serde(deserialize_with = "utils::deserializer::string_to_bool")]
    pub enable_tls: bool,
    #[serde(default)]
    pub ca_cert: String,
    #[serde(default)]
    pub cert: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub path_to_ca_cert: String,
    #[serde(default)]
    pub path_to_cert: String,
    #[serde(default)]
    pub path_to_key: String,
    #[serde(deserialize_with = "utils::deserializer::string_to_bool")]
    pub cors_allow_credentials: bool,
    #[serde(deserialize_with = "utils::deserializer::string_to_vec")]
    pub cors_allow_origins: Vec<String>,
}

#[async_trait::async_trait]
impl TlsAccept for ProxyConfig {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        // set hostname
        ssl.set_hostname("reverse-proxy")
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to set hostname: {}", e
                );
            })
            .unwrap();

        // load private key
        let key = PKey::private_key_from_pem(self.key.as_bytes())
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to parse server private key: {}", e
                );
            })
            .unwrap();

        ssl.set_private_key(&key)
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to set server private key: {}", e
                );
            })
            .unwrap();

        // load certificate chain
        let mut certs = boring::x509::X509::stack_from_pem(self.cert.as_bytes())
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to parse server certificate chain: {}", e
                );
            })
            .unwrap();

        if certs.is_empty() {
            error!(
                log_type=LogTypes::TLS_HANDSHAKE,
                "Certificate chain is empty"
            );
            panic!("Empty certificate chain");
        }

        // first cert = leaf
        let leaf = certs.remove(0);

        ssl.set_certificate(&leaf)
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to set server certificate: {}", e
                );
            })
            .unwrap();

        // remaining certs = intermediates
        for cert in certs {
            ssl.add_chain_cert(&cert)
                .inspect_err(|e| {
                    error!(
                        log_type=LogTypes::TLS_HANDSHAKE,
                        "Failed to add chain certificate: {}", e
                    );
                })
                .unwrap();
        }

        // load CA used to verify clients
        let ca_cert = boring::x509::X509::from_pem(self.ca_cert.as_bytes())
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to parse CA certificate: {}", e
                );
            })
            .unwrap();

        ssl.set_custom_verify_callback(
            SslVerifyMode::PEER,
            Self::verify_callback(ca_cert.clone()),
        );
    }
}

impl ProxyConfig {
    pub fn load_mtls_certs(&mut self) -> Result<(), String> {
        if self.ca_cert.is_empty() {
            self.ca_cert = std::fs::read_to_string(&self.path_to_ca_cert)
                .map_err(|e| format!("Failed to read CA certificate: {}", e))?;
        }

        if self.cert.is_empty() {
            self.cert = std::fs::read_to_string(&self.path_to_cert)
                .map_err(|e| format!("Failed to read certificate: {}", e))?;
        }

        if self.key.is_empty() {
            self.key = std::fs::read_to_string(&self.path_to_key)
                .map_err(|e| format!("Failed to read key: {}", e))?;
        }

        Ok(())
    }

    fn verify_callback(
        ca_cert: X509,
    ) -> Box<dyn Fn(&mut SslRef) -> Result<(), SslVerifyError> + 'static + Sync + Send> {
        Box::new(move |ssl| -> Result<(), SslVerifyError> {
            Self::verify_client_file(&ca_cert, ssl)
        })
    }

    fn verify_client_file(
        ca_cert: &X509,
        ssl: &mut TlsRef,
    ) -> Result<(), SslVerifyError> {
        let client_cert = ssl.peer_certificate().ok_or_else(|| {
            error!(
                log_type=LogTypes::TLS_HANDSHAKE,
                "Failed to get client certificate"
            );
            SslVerifyError::Invalid(SslAlert::NO_CERTIFICATE)
        })?;

        // Build CA trust store
        let mut store_builder = X509StoreBuilder::new()
            .map_err(|_| SslVerifyError::Invalid(SslAlert::INTERNAL_ERROR))?;

        store_builder
            .add_cert(ca_cert.clone())
            .map_err(|_| SslVerifyError::Invalid(SslAlert::INTERNAL_ERROR))?;

        let store = store_builder.build();

        // Create verification context
        let mut ctx = X509StoreContext::new()
            .map_err(|_| SslVerifyError::Invalid(SslAlert::INTERNAL_ERROR))?;

        // Get client-supplied intermediate chain
        let verified = if let Some(chain) = ssl.peer_cert_chain() {
            ctx.init(&store, &client_cert, chain, |c| c.verify_cert())
        } else {
            let empty_chain = Stack::<X509>::new()
                .map_err(|_| SslVerifyError::Invalid(SslAlert::INTERNAL_ERROR))?;
            ctx.init(&store, &client_cert, &empty_chain, |c| c.verify_cert())
        }
            .map_err(|_| {
                error!(
            log_type=LogTypes::TLS_HANDSHAKE,
            "Certificate verification process failed"
        );
                SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE)
            })?;

        if !verified {
            error!(
            log_type=LogTypes::TLS_HANDSHAKE,
            "Client certificate verification failed"
        );
            return Err(SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE));
        }

        info!(
        log_type=LogTypes::TLS_HANDSHAKE,
        "Client certificate verification succeeded"
    );

        Ok(())
    }
}

