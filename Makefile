run: install-concurrently
	concurrently "cd forward-proxy && cargo run" "cd reverse-proxy && cargo update && cargo run" "cd frontend && npm i && npm run dev" "cd backend && npm i && node index.js"

run-fp:
	cd forward-proxy && cargo update && cargo run

run-rp:
	cd reverse-proxy && SSLKEYLOG="ssl_keylog.txt" cargo run

run-frontend:
	cd frontend && npm i && npm run dev

run-backend:
	cd backend && npm i && node index.js
run-tests:
	cd test/cypress && npm i && npx cypress run

install-concurrently:
	which concurrently || npm i -g concurrently

# test-tls:
# 1. Make sure you have `cfssl` and `cfssljson` installed: ref <https://github.com/cloudflare/cfssl> <https://pkg.go.dev/github.com/cloudflare/cfssl/cmd/cfssljson>
# 2. Make sure you have the "*.pem" files added to your keychain for the system to trust them
# 3. Make sure you have the backend running and both the forward and reverse proxies running
#4. Make a curl request to the Forward Proxy: `curl -X POST -H "Content-Type: application/json" -H "Accept: application/json" -d "{ \"data\": \"xyz\" }" https://localhost:6191/test-endpoint`
install-curl:
	which curl || echo "Please install curl" && exit 1