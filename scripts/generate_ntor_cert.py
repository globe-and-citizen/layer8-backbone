### In case you want to generate a certificate for the ntor protocol
### To run this script, you need to provide these arguments:
###     - [REQUIRED] A 32-byte input: -v [secret] or --value [secret].
###     - [REQUIRED] The type of the input: -t [hex|dec|utf8] or --type [hex|dec|utf8].
###     - [OPTIONAL] A certificate issuer name: -iss [issuer] or --issuer [issuer].
###     - [OPTIONAL] A certificate subject name: -sub [subject] or --subject [subject].
### The script generates a certificate with a 365-day validity period.
### Example:
### ```
###     python3 generate_ntor_cert.py -t hex -v 0123456789abcdef0123456789abcdef -iss "reverse-proxy.com" -sub "ntor_server_id"
### ```

#!/usr/bin/env python3

from cryptography.hazmat.primitives.asymmetric import ed25519
from cryptography.hazmat.primitives.asymmetric import x25519
from cryptography.hazmat.primitives import serialization, hashes
from cryptography import x509
from cryptography.x509.oid import NameOID
import datetime
import sys
import argparse

def parse_input(input_type, value):
    print("value: '" + value + "'")
    if input_type == "hex":
        try:
            key_bytes = bytes.fromhex(value)
        except ValueError:
            raise ValueError("Invalid hex input.")
    elif input_type == "dec":
        try:
            key_bytes = bytes(int(b) for b in value.split(','))
        except ValueError:
            raise ValueError("Invalid decimal input. Use comma-separated values.")
    elif input_type == "utf8":
        key_bytes = value.encode("utf-8")
        print("key_bytes:", key_bytes)
        print("hex:", key_bytes.hex())
    else:
        raise ValueError("Input type must be one of: hex, dec, utf8")

    if len(key_bytes) != 32:
        raise ValueError(f"Input must be exactly 32 bytes, got {len(key_bytes)} bytes.")

    return key_bytes


def generate_certificate(public_key, private_key, issuer, subject_str):
    issuer = x509.Name([
            x509.NameAttribute(NameOID.COMMON_NAME, u"reverse_proxy.com" if not issuer else issuer),
    ])
    subject = x509.Name([
        x509.NameAttribute(NameOID.COMMON_NAME, subject_str),
    ])

    # x25519 key cannot be certificate signing key
    signing_key = ed25519.Ed25519PrivateKey.generate()

    with open("ed25519_private_key.pem", "wb") as f:
        f.write(signing_key.private_bytes(
            serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8,
            serialization.NoEncryption()
        ))

    cert = (
        x509.CertificateBuilder()
        .subject_name(subject)
        .issuer_name(issuer)
        .public_key(public_key)
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime.utcnow())
        .not_valid_after(datetime.datetime.utcnow() + datetime.timedelta(days=365))
        .sign(private_key=signing_key, algorithm=None)  # Ed25519: algorithm=None
    )

    with open("ntor_cert.pem", "wb") as f:
        f.write(cert.public_bytes(serialization.Encoding.PEM))

    print("✅ Certificate saved to ntor_cert.pem")


def main():
    parser = argparse.ArgumentParser(description="Generate Ed25519 public.pem from 32-byte input.")
    parser.add_argument("-t", "--type", choices=["hex", "dec", "utf8"], required=True, help="Input type: hex, dec, utf8")
    parser.add_argument("-v", "--value", required=True, help="Input value (string)")
    parser.add_argument("-iss", "--issuer", help="The certificate issuer")
    parser.add_argument("-sub", "--subject", help="This value is ntor 'server_id'")

    args = parser.parse_args()

    # Step 1: Define the 32-byte raw Ed25519 private key
    try:
        # raw_key = b"this is 32-byte nTorStaticSecret"
        raw_key = parse_input(args.type, args.value)
    except ValueError as e:
        print(f"❌ Error: {e}")
        return

    # Step 2: Create the Ed25519 private key
    static_private_key = x25519.X25519PrivateKey.from_private_bytes(raw_key)
    static_public_key = static_private_key.public_key()
    print("public_key", list(static_public_key.public_bytes(
                            encoding=serialization.Encoding.Raw,
                            format=serialization.PublicFormat.Raw
                        )))

    with open("x25519_public_key.pem", "wb") as f:
        pem = static_public_key.public_bytes(
            encoding=serialization.Encoding.PEM,
            format=serialization.PublicFormat.SubjectPublicKeyInfo
        )
        f.write(pem)

    with open("x25519_private_key.pem", "wb") as f:
        f.write(static_private_key.private_bytes(
            serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8,
            serialization.NoEncryption()
        ))

    generate_certificate(static_public_key, static_private_key, args.issuer, args.subject)

if __name__ == "__main__":
    main()
