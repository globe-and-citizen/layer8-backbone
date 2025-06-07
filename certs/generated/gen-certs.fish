#!/usr/bin/env fish

# This generates as CA
cfssl gencert \
        -initca ../config/ca-csr.json \
        | cfssljson -bare ca

# CA Generates & Stamps profile certificates.
cfssl gencert \
        -ca ca.pem \
        -ca-key ca-key.pem \
        -config ../config/ca-config.json \
        -profile forward_proxy ../config/forward-csr.json \
        | cfssljson -bare forward_proxy

cfssl gencert \
        -ca ca.pem \
        -ca-key ca-key.pem \
        -config ../config/ca-config.json \
        -profile reverse_proxy ../config/reverse-csr.json \
        | cfssljson -bare reverse_proxy
