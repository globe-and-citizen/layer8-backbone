use x509_parser::parse_x509_certificate;
use x509_parser::pem::parse_x509_pem;

pub fn extract_x509_pem(pem: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (_, pem) = parse_x509_pem(pem.as_bytes())?;

    // Parse the certificate
    let (_, cert) = parse_x509_certificate(&pem.contents)?;

    // Extract subject
    // let subject = cert.subject();
    // println!("📛 Subject: {}", subject);

    // Extract public key bytes
    let spki = cert.public_key().clone();
    let pubkey_bytes = spki.subject_public_key.data;

    // println!("🔑 Public Key (hex): {}", hex::encode(&pubkey_bytes));

    // Optional: show key algorithm
    // println!("🧬 Algorithm: {}", spki.algorithm.algorithm);

    Ok(pubkey_bytes.to_vec())
}