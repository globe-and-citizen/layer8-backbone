run tmux:
	tmux new-session -d -s echo-server 'cd frontend && npm run dev' \; \
	split-window -h 'cd proxy && cargo run' \; \
	attach

run:
	concurrently "cd frontend && npm run dev" "cd proxy && cargo run"