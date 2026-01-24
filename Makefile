.PHONY: up down run build logs vnc

up:
	docker compose up -d

down:
	docker compose down

build:
	cargo build --release

run: up build
	cargo run --release

logs:
	docker compose logs -f

vnc:
	@echo "Open http://localhost:7900 in browser to see Chrome (noVNC)"
