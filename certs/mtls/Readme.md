
## Generate CA (Certificate Authority):
### Generate CA private key
```shell
openssl genrsa -out ca.key 4096
```

### Generate CA certificate (self-signed, valid 10 years)
```shell
 openssl req -new -x509 -key ca.key -days 3650 -sha256 \
  -subj "/C=Vietnam/ST=Hanoi/L=Hanoi/O=GlobeAndCitizen/OU=Layer8/CN=mTLSRootCA" \
  -out ca.pem \
  -config config/ca-ext.cnf -extensions v3_ca
```

## Generate server certificate signed by CA:

### Server private key
```shell
openssl genrsa -out reverse-proxy.key 4096
```

### CSR (Certificate Signing Request)
```shell
openssl req -new -key reverse-proxy.key -out reverse-proxy.csr \
-subj "/C=Vietnam/ST=Hanoi/L=Hanoi/O=GlobeAndCitizen/OU=Layer8/CN=reverse-proxy"
```

### Sign CSR with CA
```shell
openssl x509 -req -in reverse-proxy.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
 -days 3650 -sha256 -out reverse-proxy.pem -extfile config/reverse-proxy-ext.cnf
```

## Generate client certificate signed by CA:
### Client private key
```shell
openssl genrsa -out forward-proxy.key 4096
```

# CSR
```shell
openssl req -new -key forward-proxy.key -out forward-proxy.csr \
-subj "/C=Vietnam/ST=Hanoi/L=Hanoi/O=GlobeAndCitizen/OU=Layer8/CN=forward-proxy"
```

# Sign CSR with CA
```shell
openssl x509 -req -in forward-proxy.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
-out forward-proxy.pem -days 3650 -sha256 -extfile config/forward-proxy-ext.cnf
```


