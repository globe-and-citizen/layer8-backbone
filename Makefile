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

install-curl:
	which curl || echo "Please install curl" && exit 1