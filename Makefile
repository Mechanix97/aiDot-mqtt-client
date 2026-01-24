.PHONY: up down build logs vnc

up:
	docker compose up -d --build

down:
	docker compose down

build:
	docker compose build

logs:
	docker compose logs -f

vnc:
	@echo "Open http://localhost:7900 in browser to see Chrome (noVNC)"
