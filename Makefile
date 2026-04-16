.PHONY: dev-help dev-setup dev-check dev-backend dev-frontend dev-caddy test-backend

dev-help:
	@echo "Local dev targets"
	@echo "  make dev-setup      - Create local dev config files if missing"
	@echo "  make dev-check      - Validate local HTTPS dev files/certs"
	@echo "  make dev-backend    - Run Rust backend (http://localhost:3000)"
	@echo "  make dev-frontend   - Run Angular dev server (http://localhost:4200)"
	@echo "  make dev-caddy      - Run TLS proxy (https://stars.localhost)"
	@echo "  make test-backend   - Run backend tests"

dev-setup:
	@if [ ! -f backend/.env ]; then \
		cp backend/.env.example backend/.env; \
		echo "Created backend/.env from backend/.env.example"; \
	else \
		echo "Skipped backend/.env (already exists)"; \
	fi
	@if [ ! -f frontend/src/environments/environment.ts ]; then \
		cp frontend/src/environments/environment.localhost-https.example.ts frontend/src/environments/environment.ts; \
		echo "Created frontend/src/environments/environment.ts from HTTPS template"; \
	else \
		echo "Skipped frontend/src/environments/environment.ts (already exists)"; \
	fi
	@if [ ! -f dev/Caddyfile ]; then \
		cp dev/Caddyfile.example dev/Caddyfile; \
		echo "Created dev/Caddyfile from dev/Caddyfile.example"; \
	else \
		echo "Skipped dev/Caddyfile (already exists)"; \
	fi
	@mkdir -p dev/certs
	@echo "Ensured dev/certs directory exists"

dev-check:
	@test -f backend/.env || (echo "Missing backend/.env" && exit 1)
	@test -f dev/Caddyfile || (echo "Missing dev/Caddyfile (copy from dev/Caddyfile.example)" && exit 1)
	@test -f dev/certs/stars.localhost+3.pem || (echo "Missing TLS cert: dev/certs/stars.localhost+3.pem" && exit 1)
	@test -f dev/certs/stars.localhost+3-key.pem || (echo "Missing TLS key: dev/certs/stars.localhost+3-key.pem" && exit 1)
	@echo "Local HTTPS prerequisites look good."

dev-backend:
	cd backend && cargo run

dev-frontend:
	cd frontend && npm run start

dev-caddy:
	caddy run --config dev/Caddyfile

test-backend:
	cargo test -p stars-reborn-backend

