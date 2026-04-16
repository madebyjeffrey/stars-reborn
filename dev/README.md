# Local HTTPS Dev Setup (IDE Friendly)

This setup uses `mkcert` + `Caddy` so your app runs at `https://stars.localhost` while frontend/backend keep their normal dev servers.

## What you get

- One HTTPS origin for browser testing: `https://stars.localhost`
- API routed to backend via `/api/*`
- Works well with secure cookies and OAuth callback flows

## Prerequisites (macOS)

```zsh
brew install mkcert caddy nss
mkcert -install
```

## 1) Create local certs

```zsh
mkdir -p /Users/jdrake/RustroverProjects/stars-reborn/dev/certs
cd /Users/jdrake/RustroverProjects/stars-reborn/dev/certs
mkcert stars.localhost localhost 127.0.0.1 ::1
```

Expected files (names may vary slightly if regenerated):

- `stars.localhost+3.pem`
- `stars.localhost+3-key.pem`

## 2) Backend env template

Copy the template and set real values:

```zsh
cd /Users/jdrake/RustroverProjects/stars-reborn/backend
cp .env.example .env
```

Template defaults now assume HTTPS local proxy:

- `FRONTEND_URL=https://stars.localhost`
- `DISCORD_REDIRECT_URL=https://stars.localhost/api/auth/discord/callback`

## 3) Frontend env template

Copy the HTTPS template over local environment file:

```zsh
cd /Users/jdrake/RustroverProjects/stars-reborn/frontend/src/environments
cp environment.localhost-https.example.ts environment.ts
```

## 4) Caddy config template

Copy the example and adjust cert paths if needed:

```zsh
cd /Users/jdrake/RustroverProjects/stars-reborn/dev
cp Caddyfile.example Caddyfile
```

## 5) Run from IDEs (recommended)

Before creating run configurations, bootstrap missing local files once:

```zsh
cd /Users/jdrake/RustroverProjects/stars-reborn
make dev-setup
```

Create three run configurations:

- **Backend**
  - Working directory: `/Users/jdrake/RustroverProjects/stars-reborn`
  - Command: `make dev-backend`
- **Frontend**
  - Working directory: `/Users/jdrake/RustroverProjects/stars-reborn`
  - Command: `make dev-frontend`
- **Caddy**
  - Working directory: `/Users/jdrake/RustroverProjects/stars-reborn`
  - Command: `make dev-caddy`

Optional validation target before launching:

```zsh
cd /Users/jdrake/RustroverProjects/stars-reborn
make dev-check
```

Open the app at `https://stars.localhost`.

## Discord OAuth callback

Set this exact redirect URI in Discord developer settings:

- `https://stars.localhost/api/auth/discord/callback`

## Troubleshooting

- Browser warns about cert: rerun `mkcert -install`, restart browser.
- Caddy fails to start: verify cert paths in `dev/Caddyfile`.
- OAuth callback mismatch: verify backend `.env` and Discord app redirect URI are identical.
- Frontend still calls `http://localhost:3000`: re-check `frontend/src/environments/environment.ts`.
