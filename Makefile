run-fp:
	cd forward-proxy && cargo run

run-rp:
	cd reverse-proxy && cargo run
run-frontend:
	cd frontend && npm i && npm run dev

run-backend:
	cd backend && npm i && node index.js