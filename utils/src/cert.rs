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
