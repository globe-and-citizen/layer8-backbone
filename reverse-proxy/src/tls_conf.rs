use boring::{
    pkey::{PKey, Public},
    ssl::{SslAlert, SslRef, SslVerifyError, SslVerifyMode},
};
use log::debug;
use pingora::{listeners::TlsAccept, protocols::tls::TlsRef};

pub struct TlsConfig;

#[async_trait::async_trait]
impl TlsAccept for TlsConfig {
    async fn certificate_callback(&self, ssl: &mut TlsRef) {
        let ca_pem = cert::ca_pem();
        let server_pem = cert::cert();
        let server_key = cert::key();

        // set the hostname for the SSL context
        ssl.set_hostname("localhost")
            .inspect_err(|e| {
                log::error!("Failed to set hostname: {}", e);
            })
            .unwrap();

        // provide the private key
        {
            let key = PKey::private_key_from_pem(&server_key)
                .inspect_err(|e| {
                    log::error!("Failed to parse server private key: {}", e);
                })
                .unwrap();
            ssl.set_private_key(&key)
                .inspect_err(|e| {
                    log::error!("Failed to set server private key: {}", e);
                })
                .unwrap();
        }

        // provide the certificate chain file
        {
            let cert = boring::x509::X509::from_pem(&server_pem)
                .inspect_err(|e| {
                    log::error!("Failed to parse server certificate: {}", e);
                })
                .unwrap();

            ssl.set_certificate(&cert)
                .inspect_err(|e| {
                    log::error!("Failed to set server certificate: {}", e);
                })
                .unwrap();
        }

        // the CA certificate is used to verify the client certificate
        let ca_cert = boring::x509::X509::from_pem(&ca_pem)
            .inspect_err(|e| {
                log::error!("Failed to parse CA certificate: {}", e);
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
            log::error!("SSL verify mode is not set to PEER, cannot verify client certificate");
            return Err(SslVerifyError::Invalid(SslAlert::INTERNAL_ERROR));
        }

        let client_cert = match ssl.peer_certificate() {
            Some(val) => val,
            None => {
                log::error!("Failed to get client certificate");
                return Err(SslVerifyError::Invalid(SslAlert::NO_CERTIFICATE));
            }
        };

        // Making sure the client CN is "forward_proxy" in debug logs
        debug!("Debug Client certificate: {:?}", client_cert.subject_name());

        // Verify the client certificate against the server's CA
        if !client_cert.verify(&server_ca_public_key).unwrap() {
            log::error!("Client certificate verification failed");
            return Err(SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE));
        }
        debug!("Client certificate verification succeeded");

        Ok(())
    }
}

mod cert {
    pub fn ca_pem() -> Vec<u8> {
        std::fs::read(std::env::var("PATH_TO_CA_CERT").expect("Failed to read CA PEM file"))
            .expect("Failed to read CA PEM file")
    }

    pub fn cert() -> Vec<u8> {
        std::fs::read(std::env::var("PATH_TO_CERT").expect("Failed to read cert PEM file"))
            .expect("Failed to read cert PEM file")
    }

    pub fn key() -> Vec<u8> {
        std::fs::read(std::env::var("PATH_TO_KEY").expect("Failed to read key PEM file"))
            .expect("Failed to read key PEM file")
    }
}
