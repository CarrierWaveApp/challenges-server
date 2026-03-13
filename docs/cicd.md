# CI/CD Pipeline Design

## Overview

Three-stage pipeline: **CI (test)** → **Release (build)** → **Deploy (Ansible)**.
E2E tests run against a real Postgres instance in Docker to validate all endpoints before release.

```
Push/PR to main ──► CI Workflow
                     ├─ cargo fmt --check
                     ├─ cargo clippy
                     ├─ cargo test (unit)
                     ├─ Build frontend
                     └─ E2E tests (Docker Compose: Postgres + server)
                          └─ tests/e2e.sh hits every endpoint

Tag push (v*) ────► Release Workflow (existing, unchanged)
                     └─ Build musl binary → GitHub Release

Release published ► Deploy Workflow
                     └─ Ansible playbook via SSH
                          ├─ Download release archive
                          ├─ Run migrations
                          ├─ Restart systemd service
                          └─ Smoke test /v1/health
```

## Pipeline Stages

### 1. CI Workflow (`.github/workflows/ci.yml`)

**Triggers:** Push to `main`, all pull requests.

| Job | Purpose | Services |
|-----|---------|----------|
| `lint` | `cargo fmt --check` + `cargo clippy` | None |
| `test` | `cargo test` (unit + integration) | Postgres 16 |
| `e2e` | Build server, start it, run endpoint tests | Postgres 16 |

The `e2e` job:
1. Builds the frontend (`npm ci && npm run build`)
2. Builds the server in debug mode
3. Runs migrations via the server startup
4. Executes `tests/e2e.sh` against `http://localhost:8080`
5. Validates every endpoint returns expected status codes and response shapes

### 2. Release Workflow (`.github/workflows/release.yml`)

**Existing workflow, unchanged.** Triggers on `v*` tags, builds a musl static binary, creates a GitHub Release with the archive.

### 3. Deploy Workflow (`.github/workflows/deploy.yml`)

**Triggers:** GitHub Release published.

1. Downloads the release archive
2. Runs the Ansible playbook targeting the production server
3. Ansible handles: stop service → deploy binary + migrations + frontend → run migrations → start service → smoke test

## E2E Test Design

### Approach

A bash script (`tests/e2e.sh`) that exercises every public endpoint. Runs in CI with a real Postgres database. No mocks.

### What it tests

| Endpoint | Validation |
|----------|------------|
| `GET /v1/health` | 200, JSON body has `status: "ok"` |
| `GET /v1/challenges` | 200, returns JSON array |
| `GET /v1/programs` | 200, returns JSON array |
| `GET /v1/pota/stats/rankings/activators` | 200, returns JSON |
| `GET /v1/pota/stats/status` | 200 |
| `GET /v1/rbn/spots` | 200 (when RBN disabled, still returns empty) |
| `GET /v1/rbn/stats` | 200 |
| `POST /v1/admin/challenges` | 201 with admin token, 401 without |
| `GET /v1/challenges/{id}` | 200 for created challenge |
| `POST /v1/challenges/{id}/join` | Join flow with device token |
| `POST /v1/challenges/{id}/progress` | Progress reporting |
| `GET /v1/challenges/{id}/leaderboard` | 200 |
| `DELETE /v1/challenges/{id}/leave` | Leave flow |
| `DELETE /v1/admin/challenges/{id}` | Admin cleanup |

### Test characteristics

- **Idempotent:** Creates its own test data, cleans up after
- **Fast:** ~5-10 seconds total (just HTTP calls)
- **Deterministic:** No external dependencies beyond Postgres

## Ansible Deployment

The Ansible playbook lives in a **separate repository** (configured via the `ANSIBLE_REPO` repository variable in GitHub). The deploy workflow checks it out at runtime.

### How the deploy workflow integrates your playbook

1. Checks out your Ansible repo using a deploy key
2. Runs `ansible-playbook playbook.yml` with `--extra-vars`:
   - `release_version` — the git tag (e.g. `v0.4.39`)
   - `github_repo` — `CarrierWaveApp/challenges-server` (so the playbook can construct the download URL)
3. After Ansible completes, the workflow smoke-tests `/v1/health`
4. On success, posts a deploy annotation to Grafana Cloud

### What your Ansible playbook needs to handle

Your existing playbook should accept these extra vars and handle:

1. **Download** the release archive from GitHub:
   `https://github.com/{{ github_repo }}/releases/download/{{ release_version }}/activities-server-{{ release_version }}-x86_64-linux.tar.gz`
2. **Extract** to the deploy directory (e.g. `/opt/challenges-server/`)
3. **Run migrations** (`sqlx migrate run` or let the server auto-run on startup)
4. **Restart** the systemd service
5. **Verify** the service started (health check)

### Grafana integration

The server already exports Prometheus metrics on `/metrics`. Two integration points:

1. **Metrics scraping** — Configure Grafana Cloud (or Grafana Alloy/Agent running on your server) to scrape `/metrics`. This is a one-time setup outside the pipeline.
2. **Deploy annotations** — The deploy workflow posts an annotation to Grafana on each successful deploy, marking the timestamp on your dashboards. Requires `GRAFANA_URL` and `GRAFANA_API_KEY` secrets.

## Required Secrets

| Secret | Used by | Purpose |
|--------|---------|---------|
| `DEPLOY_SSH_KEY` | Deploy workflow | SSH key for Ansible to reach production server |
| `DEPLOY_HOST` | Deploy workflow | Target server hostname/IP (for SSH keyscan + smoke test) |
| `ANSIBLE_REPO_DEPLOY_KEY` | Deploy workflow | SSH deploy key to clone the Ansible repo |
| `GRAFANA_API_KEY` | Deploy workflow | Post deploy annotations to Grafana Cloud |
| `GRAFANA_URL` | Deploy workflow | Grafana Cloud instance URL |
| `ADMIN_TOKEN` | E2E tests | Admin endpoint testing (hardcoded in CI env) |

### Required Repository Variables

| Variable | Used by | Purpose |
|----------|---------|---------|
| `ANSIBLE_REPO` | Deploy workflow | Ansible repo (e.g. `CarrierWaveApp/infrastructure`) |
| `PRODUCTION_URL` | Deploy workflow | Production base URL for smoke test |

## File inventory

| File | Purpose |
|------|---------|
| `.github/workflows/ci.yml` | CI: lint, test, E2E |
| `.github/workflows/deploy.yml` | CD: Ansible deploy on release |
| `tests/e2e.sh` | E2E test script |
| *(separate repo)* | Ansible playbook — checked out at deploy time |
