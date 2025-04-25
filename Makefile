run:
	concurrently "cd forward-proxy && cargo run" "cd reverse-proxy && cargo run" "cd frontend && npm i && npm run dev" "cd backend && npm i && node index.js"

run-fp:
	cd forward-proxy && cargo run

run-rp:
	cd reverse-proxy && cargo run

run-frontend:
	cd frontend && npm i && npm run dev

run-backend:
	cd backend && npm i && node index.js