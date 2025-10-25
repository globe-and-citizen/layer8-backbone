use boring::{
    pkey::{PKey, Public},
    ssl::{SslAlert, SslRef, SslVerifyError, SslVerifyMode},
};
use pingora::{listeners::TlsAccept, protocols::tls::TlsRef};
use serde::Deserialize;
use tracing::{debug, error, info};
use crate::handler::common::consts::LogTypes;

#[derive(Debug, Deserialize, Clone)]
pub struct TlsConfig {
    #[serde(deserialize_with = "utils::deserializer::string_to_bool")]
    pub enable_tls: bool,
    pub ca_cert: String,
    pub cert: String,
    pub key: String,
}

#[async_trait::async_trait]
impl TlsAccept for TlsConfig {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        // set the hostname for the SSL context
        ssl.set_hostname("localhost")
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to set hostname: {}", e
                );
            })
            .unwrap();

        // provide the private key
        {
            let key = PKey::private_key_from_pem(&self.key.clone().into_bytes())
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
        }

        // provide the certificate chain file
        {
            let cert = boring::x509::X509::from_pem(&self.cert.clone().into_bytes())
                .inspect_err(|e| {
                    error!(
                        log_type=LogTypes::TLS_HANDSHAKE,
                        "Failed to parse server certificate: {}", e
                    );
                })
                .unwrap();

            ssl.set_certificate(&cert)
                .inspect_err(|e| {
                    error!(
                        log_type=LogTypes::TLS_HANDSHAKE,
                        "Failed to set server certificate: {}", e
                    );
                })
                .unwrap();
        }

        // the CA certificate is used to verify the client certificate
        let ca_cert = boring::x509::X509::from_pem(&self.ca_cert.clone().into_bytes())
            .inspect_err(|e| {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to parse CA certificate: {}", e
                );
            })
            .unwrap();

        ssl.set_custom_verify_callback(
            SslVerifyMode::PEER,
            Self::verify_callback(ca_cert.public_key().unwrap()),
        );
    }
}

impl TlsConfig {
    fn verify_callback(
        ca_cert_pub_key: PKey<Public>,
    ) -> Box<dyn Fn(&mut SslRef) -> Result<(), SslVerifyError> + 'static + Sync + Send> {
        Box::new(move |ssl| -> Result<(), SslVerifyError> {
            Self::verify_client_file(&ca_cert_pub_key, ssl)
        })
    }

    fn verify_client_file(
        server_ca_public_key: &PKey<Public>,
        ssl: &mut TlsRef,
    ) -> Result<(), SslVerifyError> {
        if ssl.verify_mode() != SslVerifyMode::PEER {
            error!(
                log_type=LogTypes::TLS_HANDSHAKE,
                "SSL verify mode is not set to PEER, cannot verify client certificate"
            );
            return Err(SslVerifyError::Invalid(SslAlert::INTERNAL_ERROR));
        }

        let client_cert = match ssl.peer_certificate() {
            Some(val) => val,
            None => {
                error!(
                    log_type=LogTypes::TLS_HANDSHAKE,
                    "Failed to get client certificate"
                );
                return Err(SslVerifyError::Invalid(SslAlert::NO_CERTIFICATE));
            }
        };

        // Making sure the client CN is "forward_proxy" in debug logs
        debug!("Debug Client certificate: {:?}", client_cert.subject_name());

        // Verify the client certificate against the server's CA
        if !client_cert.verify(&server_ca_public_key).unwrap_or_default() {
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
