run-fp:
	cd forward-proxy && cargo update && cargo run

run-frontend:
	cd frontend && npm i && npm run dev

run-backend:
	cd backend && npm i && node index.js