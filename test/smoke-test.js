'use strict';

/**
 * Layer8 smoke e2e test for Forward Proxy (FP) and Reverse Proxy (RP).
 *
 * Assertions:
 *  1. FP /healthcheck returns 200
 *  2. Mock-auth /healthcheck returns 200
 *  3. Backend /healthcheck returns 200
 *  4. FP→RP init-tunnel succeeds (proves the full mTLS chain is working)
 *  5. Proxy request via interceptor fetch succeeds
 *
 * Run after `npm run smoke:wait` to ensure all services are ready.
 */

const http = require('http');
const crypto = require('crypto');

const FP_URL = process.env.FP_URL || 'http://localhost:6191';
const BACKEND_URL = process.env.BACKEND_URL || 'http://localhost:3000';
const MOCK_AUTH_URL = process.env.MOCK_AUTH_URL || 'http://localhost:5001';
const PROXY_TARGET_URL = process.env.PROXY_TARGET_URL || 'http://spa-backend:3000';

// The backend_url sent to FP in init-tunnel must match RP's NTOR_SERVER_ID
const RP_BACKEND_URL = process.env.RP_BACKEND_URL || 'https://reverse-proxy:6193';

// Retry configuration for the init-tunnel check (RP may still be starting up)
const INIT_TUNNEL_MAX_ATTEMPTS = parseInt(process.env.INIT_TUNNEL_RETRIES || '20', 10);
const INIT_TUNNEL_RETRY_DELAY_MS = parseInt(process.env.INIT_TUNNEL_RETRY_DELAY_MS || '3000', 10);

function httpRequest(method, url, body) {
    return new Promise((resolve, reject) => {
        const parsed = new URL(url);
        const options = {
            hostname: parsed.hostname,
            port: parsed.port || 80,
            path: parsed.pathname + parsed.search,
            method,
            headers: {},
        };

        const bodyStr = body ? (typeof body === 'string' ? body : JSON.stringify(body)) : null;

        if (bodyStr) {
            options.headers['Content-Type'] = 'application/json';
            options.headers['Content-Length'] = Buffer.byteLength(bodyStr);
        }

        const req = http.request(options, (res) => {
            let data = '';
            res.on('data', (chunk) => { data += chunk; });
            res.on('end', () => resolve({ status: res.statusCode, body: data }));
        });

        req.on('error', reject);

        if (bodyStr) {
            req.write(bodyStr);
        }
        req.end();
    });
}

async function checkHealthcheck(name, baseUrl) {
    try {
        const res = await httpRequest('GET', `${baseUrl}/healthcheck`, null);
        if (res.status === 200) {
            console.log(`  ✓ ${name} /healthcheck → ${res.status}`);
            return true;
        }
        console.error(`  ✗ ${name} /healthcheck → ${res.status} (expected 200)`);
        return false;
    } catch (err) {
        console.error(`  ✗ ${name} /healthcheck → error: ${err.message}`);
        return false;
    }
}

async function tryInitTunnel() {
    // Generate a random X25519 key pair and send the public key to FP
    const { publicKey } = crypto.generateKeyPairSync('x25519');
    const jwk = publicKey.export({ format: 'jwk' });
    const rawPubKey = Buffer.from(jwk.x, 'base64url');
    if (rawPubKey.length !== 32) {
        throw new Error(`Unexpected X25519 public key length: ${rawPubKey.length}`);
    }
    const body = JSON.stringify({ public_key: Array.from(rawPubKey) });

    // Do not URL-encode the backend_url value: the FP's query parser uses
    // splitn(2, '=') and passes the raw value to Url::parse without decoding.
    const url = `${FP_URL}/init-tunnel?backend_url=${RP_BACKEND_URL}`;
    const res = await httpRequest('POST', url, body);
    return res;
}

async function checkInitTunnel() {
    for (let attempt = 1; attempt <= INIT_TUNNEL_MAX_ATTEMPTS; attempt++) {
        try {
            const res = await tryInitTunnel();
            if (res.status === 200) {
                console.log(`  ✓ FP→RP init-tunnel → ${res.status} (attempt ${attempt})`);
                return true;
            }
            console.log(`  … FP→RP init-tunnel → ${res.status} (attempt ${attempt}/${INIT_TUNNEL_MAX_ATTEMPTS})`);
        } catch (err) {
            console.log(`  … FP→RP init-tunnel → error: ${err.message} (attempt ${attempt}/${INIT_TUNNEL_MAX_ATTEMPTS})`);
        }

        if (attempt < INIT_TUNNEL_MAX_ATTEMPTS) {
            await new Promise((r) => setTimeout(r, INIT_TUNNEL_RETRY_DELAY_MS));
        }
    }
    console.error(`  ✗ FP→RP init-tunnel failed after ${INIT_TUNNEL_MAX_ATTEMPTS} attempts`);
    return false;
}

async function checkProxyRequest() {
    try {
        const interceptorWasm = await import('l8-intercept');
        interceptorWasm.initEncryptedTunnel(FP_URL, [interceptorWasm.ServiceProvider.new(PROXY_TARGET_URL)]);

        const response = await interceptorWasm.fetch(`${PROXY_TARGET_URL}/healthcheck`);
        if (response.status === 200) {
            console.log(`  ✓ Interceptor proxy healthcheck → ${response.status}`);
            return true;
        }
        console.error(`  ✗ Interceptor proxy healthcheck → ${response.status} (expected 200)`);
        return false;
    } catch (err) {
        console.error(`  ✗ Interceptor proxy healthcheck → error: ${err.message}`);
        return false;
    }
}

async function main() {
    let passed = 0;
    let failed = 0;

    function record(ok) {
        if (ok) passed++; else failed++;
    }

    console.log('\nLayer8 smoke e2e test\n');

    console.log('Service healthchecks:');
    record(await checkHealthcheck('Forward Proxy', FP_URL));
    record(await checkHealthcheck('Mock Auth', MOCK_AUTH_URL));
    record(await checkHealthcheck('Backend', BACKEND_URL));

    console.log('\nProxy chain:');
    const initTunnelOk = await checkInitTunnel();
    record(initTunnelOk);

    console.log('\nProxy smoke:');
    if (initTunnelOk) {
        record(await checkProxyRequest());
    } else {
        console.error('  ✗ Interceptor proxy healthcheck → skipped because init-tunnel failed');
        record(false);
    }

    console.log(`\nResults: ${passed} passed, ${failed} failed\n`);
    process.exit(failed > 0 ? 1 : 0);
}

main().catch((err) => {
    console.error('Unexpected error:', err);
    process.exit(1);
});
