use std::sync::Arc;

use async_trait::async_trait;
use boring::x509::X509;
use bytes::Bytes;
use log::info;
use pingora::upstreams::peer::PeerOptions;
use pingora::utils::tls::CertKey;
use pingora::OrErr;
use pingora::http::{ResponseHeader, StatusCode};
use pingora::listeners::tls::TLS_CONF_ERR;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora_router::ctx::{Layer8Context, Layer8ContextTrait};
use pingora_router::router::Router;

pub struct ForwardProxy<T> {
    router: Router<T>,
}

impl<T> ForwardProxy<T> {
    pub fn new(router: Router<T>) -> Self {
        ForwardProxy { router }
    }
}

#[async_trait]
impl<T: Sync> ProxyHttp for ForwardProxy<T> {
    type CTX = Layer8Context;

    fn new_ctx(&self) -> Self::CTX {
        Layer8Context::default()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        // testing certs data; fixme to be dynamic

        // mTLS Steps:
        // 1. Client connects to server
        // 2. Server presents its TLS certificate
        // 3. Client verifies the server's certificate
        // 4. Client presents its TLS certificate
        // 5. Server verifies the client's certificate
        // 6. Server grants access
        // 7. Client and server exchange information over encrypted TLS connection
        //
        // Code below is for step 4(this is a client to RP), presenting the client's TLS certificate.
        let mut peer = HttpPeer::new(
            String::from("localhost:6193"),
            true,
            "localhost".to_string(),
        );

        {
            let cert = X509::from_pem(&certs::cert())
                .or_err(TLS_CONF_ERR, "Failed to load FP's certificate")?;

            let ca_cert = X509::from_pem(&certs::ca_pem())
                .or_err(TLS_CONF_ERR, "Failed to load CA certificate")?;

            let key = boring::pkey::PKey::private_key_from_pem(&certs::key())
                .or_err(TLS_CONF_ERR, "Failed to load private key")?;

            // The certificate to present in mTLS connections to upstream
            // The organization implementing mTLS acts as its own certificate authority.
            let cert_key = CertKey::new(vec![cert], key);

            // Providing Peer Options
            let mut peer_options = PeerOptions::new();
            {
                peer_options.verify_cert = true; // Verify the server's certificate
                peer_options.ca = Some(Arc::new(Box::new([ca_cert])));
                peer_options.verify_hostname = true; // Whether to check if upstream server cert's Host matches the SNI
            }

            peer.client_cert_key = Some(Arc::new(cert_key));
            peer.options = peer_options;
        }

        Ok(Box::new(peer))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        // create Context
        ctx.update(session).await?;
        let request_summary = format!(
            "{} {}",
            session.req_header().method,
            session.req_header().uri.to_string()
        );
        println!();
        info!("[REQUEST {}] {:?}", request_summary, ctx.request);
        info!(
            "[REQUEST {}] Decoded body: {}",
            request_summary,
            String::from_utf8_lossy(&*ctx.get_request_body())
        );
        println!();

        let handler_response = self.router.call_handler(ctx).await;
        if handler_response.status == StatusCode::NOT_FOUND && handler_response.body == None {
            return Ok(false);
        }

        // set headers
        let mut header = ResponseHeader::build(handler_response.status, None)?;
        let response_header = ctx.get_response_header().clone();
        for (key, val) in response_header.iter() {
            header.insert_header(key.clone(), val.clone()).unwrap();
        }

        let mut response_bytes = vec![];
        if let Some(body_bytes) = handler_response.body {
            header
                .insert_header("Content-length", &body_bytes.len().to_string())
                .unwrap();
            response_bytes = body_bytes;
        };

        session.write_response_header_ref(&header).await?;

        println!();
        info!(
            "[RESPONSE {}] Header: {:?}",
            request_summary, header.headers
        );
        info!(
            "[RESPONSE {}] Body: {}",
            request_summary,
            String::from_utf8_lossy(&*response_bytes)
        );
        println!();

        // Write the response body to the session after setting headers
        session
            .write_response_body(Some(Bytes::from(response_bytes)), true)
            .await?;
        Ok(true)
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        upstream_response.insert_header("Access-Control-Allow-Origin", "*")?;
        upstream_response.insert_header("Access-Control-Allow-Methods", "GET, POST")?;
        upstream_response.insert_header("Access-Control-Allow-Headers", "Content-Type")?;
        Ok(())
    }
}

mod certs {
    use std::fs;

    pub fn ca_pem() -> Vec<u8> {
        fs::read(std::env::var("PATH_TO_CA_CERT").expect("PATH_TO_CA_PERM must be set"))
            .expect("Failed to read CA PEM file")
    }

    pub fn cert() -> Vec<u8> {
        fs::read(std::env::var("PATH_TO_CERT").expect("PATH_TO_CERT must be set"))
            .expect("Failed to read certificate file")
    }

    pub fn key() -> Vec<u8> {
        fs::read(std::env::var("PATH_TO_KEY").expect("PATH_TO_KEY must be set"))
            .expect("Failed to read key file")
    }
}
