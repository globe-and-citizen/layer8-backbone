# Working with self-signed tls certificates

## Setup

Under the `certs` directory, you can find a self-signed certificate and key pair. These are used to secure the connection between the client and server.

You can however generate a new set of certificates using the following command:

```bash
fish certs/generated/gen-certs.fish
```

This generates a CA certificate and key pair, a certificate key pair for the reverse proxy and another for the forward proxy.

## Usage Tips

We will be using the frontend and backend test code to try out our tls connection.

We first have to make our system trust the root ca certificate(certs/generates/ca.pem).

Look for instructions on how to make your system trust you new self-signed certificate:

- For macOs, you have to save it in the keychain and set it to always trust.
- For Windows, Linux and other systems, the setup can be done by going to settings and uploading the certificate to the trusted root certification authorities store.
