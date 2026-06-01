use crate::handler::common::consts::LogTypes;
use boring::stack::Stack;
use boring::x509::X509;
use boring::x509::store::X509StoreBuilder;
use boring::{
    ssl::{SslAlert, SslRef, SslVerifyError, SslVerifyMode},
    x509::X509StoreContext,
};
use pingora::{listeners::TlsAccept, protocols::tls::TlsRef};
use std::sync::Arc;
use tracing::{error, info};
use utils::cert::TLSCredentials;

/// TLS configuration for the reverse-proxy server side.
///
/// Used in the TLS accept callback to:
/// - set SNI/hostname;
/// - provide the current server certificate and private key;
/// - configure client certificate verification.
pub struct TLSServerConfig {
    /// Expected host name (SNI) set on the TLS session.
    pub host_name: String,
    /// Atomically updated server TLS credentials (leaf/key/intermediates/CA).
    pub tls_credentials: Arc<TLSCredentials>,
}

/// impl TlsAccept for TLSServerConfig means TLSServerConfig provides the server-side TLS accept behavior required by Pingora.
///
/// In this file, that implementation defines certificate_callback, which runs during handshake to:
/// - set SNI/hostname on the TLS session,
/// - load and apply the current server key/certificate chain,
/// - install custom client certificate verification logic.
#[async_trait::async_trait]
impl TlsAccept for TLSServerConfig {
    /// TLS accept callback invoked during the TLS handshake.
    ///
    /// Performs the following steps:
    /// - Sets the expected hostname (SNI) on the TLS session;
    /// - Loads the current server TLS credentials (private key, leaf certificate, intermediate chain);
    /// - Installs a custom client certificate verification callback using the configured CA.
    ///
    /// # Panics
    /// Panics if any of the TLS configuration steps fail (via `unwrap()`), logging the error beforehand.
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        // set hostname
        ssl.set_hostname(&self.host_name)
            .inspect_err(|e| {
                error!(
                    log_type = LogTypes::TLS_HANDSHAKE,
                    "Failed to set hostname: {}", e
                );
            })
            .unwrap();

        // load current certificate/key atomically
        let cert_key = self.tls_credentials.cert_key.load_full();

        // set private key
        ssl.set_private_key(cert_key.key())
            .inspect_err(|e| {
                error!(
                    log_type = LogTypes::TLS_HANDSHAKE,
                    "Failed to set server private key: {}", e
                );
            })
            .unwrap();

        // leaf certificate
        ssl.set_certificate(cert_key.leaf())
            .inspect_err(|e| {
                error!(
                    log_type = LogTypes::TLS_HANDSHAKE,
                    "Failed to set server certificate: {}", e
                );
            })
            .unwrap();

        // intermediate chain
        for cert in cert_key.intermediates() {
            ssl.add_chain_cert(cert)
                .inspect_err(|e| {
                    error!(
                        log_type = LogTypes::TLS_HANDSHAKE,
                        "Failed to add chain certificate: {}", e
                    );
                })
                .unwrap();
        }

        // CA used for client verification
        ssl.set_custom_verify_callback(
            SslVerifyMode::PEER,
            Self::verify_callback(self.tls_credentials.ca_cert.clone()),
        );
    }
}

#[allow(clippy::type_complexity)]
impl TLSServerConfig {
    fn verify_callback(
        ca_cert: X509,
    ) -> Box<dyn Fn(&mut SslRef) -> Result<(), SslVerifyError> + 'static + Sync + Send> {
        Box::new(move |ssl| -> Result<(), SslVerifyError> {
            Self::verify_client_file(&ca_cert, ssl)
        })
    }

    /// Verifies the client certificate against the configured CA certificate.
    ///
    /// This function:
    /// - retrieves the peer (client) certificate from the TLS session;
    /// - builds an in-memory trust store containing the provided CA certificate;
    /// - initializes an X\.509 verification context;
    /// - verifies the client certificate using the peer-provided chain (if present).
    ///
    /// Returns `Ok(())` when verification succeeds, otherwise returns
    /// `SslVerifyError` mapped to an appropriate TLS alert.
    fn verify_client_file(ca_cert: &X509, ssl: &mut TlsRef) -> Result<(), SslVerifyError> {
        let client_cert = ssl.peer_certificate().ok_or_else(|| {
            error!(
                log_type = LogTypes::TLS_HANDSHAKE,
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
                log_type = LogTypes::TLS_HANDSHAKE,
                "Certificate verification process failed"
            );
            SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE)
        })?;

        if !verified {
            error!(
                log_type = LogTypes::TLS_HANDSHAKE,
                "Client certificate verification failed"
            );
            return Err(SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE));
        }

        info!(
            log_type = LogTypes::TLS_HANDSHAKE,
            "Client certificate verification succeeded"
        );

        Ok(())
    }
}
