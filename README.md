# OMP Session Visualizer

[![Rust](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](https://www.rust-lang.org)
[![Nix](https://img.shields.io/badge/nix-flakes-blue.svg)](https://nixos.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

> AI coding agent session visualization — browse, search, and explore omp coding-agent sessions through an interactive timeline interface.

Built for **信息系统开发** 期末大作业 | 班级: 25262-2091186-001

---

## Features

- **Session Browser** — List all omp sessions with title, directory, model, message count, and subagent info
- **Interactive Timeline** — Capsule-seed timeline view of session entries with color-coded message/event types
- **Conversation View** — Full message rendering with tool-use, thinking blocks, and image content
- **Subagent Tracks** — Nested subagent sessions appear as parallel tracks in the timeline
- **Full-Text Search** — Search across messages, raw events, and subagent tracks via SQLite FTS5
- **Session Index Cache** — Fingerprint-based caching for instant session listing and timeline boot
- **Gzip Compression** — Compressed API payloads for fast timeline loading
- **Multi-Agent Format** — Renders omp's JSONL session format into minelogue's frontend wire format

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Browser                              │
│              (Minelogue Frontend: JS/CSS/HTML)              │
└─────────────────────────┬───────────────────────────────────┘
                          │  JSON API + Static Files
┌─────────────────────────▼───────────────────────────────────┐
│                 OMP Visualizer Backend                       │
│                  (Rust + Axum + Tera)                        │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐ │
│  │  Parser  │  │  Store   │  │  Index   │  │  Templates  │ │
│  │ JSONL→   │  │ Session  │  │ SQLite   │  │ Dashboard/ │ │
│  │ Models   │  │ Listing  │  │ Cache    │  │ Conv Pages │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬──────┘ │
│       │              │              │               │       │
│  ┌────▼──────────────▼──────────────▼───────────────▼──────┐│
│  │                    API Routes                           ││
│  │  /api/sessions  /api/conversation  /api/timeline       ││
│  │  /api/track     /api/message      /api/search          ││
│  └────────────────────────────────────────────────────────┘│
└─────────────────────────┬───────────────────────────────────┘
                          │  Reads JSONL
┌─────────────────────────▼───────────────────────────────────┐
│              ~/.omp/agent/sessions/                          │
│         (omp coding-agent session files)                     │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# Build with Nix
nix build .#default

# Run
./result/bin/omp-visualizer

# Open browser
open http://localhost:3000
```

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/` | GET | Dashboard page (server-rendered) |
| `/conversation/{id}` | GET | Conversation page with timeline |
| `/api/sessions` | GET | List all sessions (`?agent=omp&q=&directory=`) |
| `/api/conversation/omp/{id}` | GET | Slim conversation export (`?slim=true`) |
| `/api/conversation/omp/{id}/timeline` | GET | Timeline boot payload (ETag + gzip) |
| `/api/conversation/omp/{id}/track/{track}` | GET | Single track payload |
| `/api/conversation/omp/{id}/message` | GET | Single message (`?track_id=&line_number=`) |
| `/api/conversation/omp/{id}/raw_event` | GET | Single raw event (`?jsonl_file=&line_number=`) |
| `/api/conversation/omp/{id}/search` | GET | Full-text search (`?q=&scope=`) |
| `/static/*` | GET | Static assets (JS/CSS) |

## Build Instructions

### Cargo

```bash
cd backend
cargo build --release
./target/release/omp-visualizer
```

### Nix

```bash
# Build the binary
nix build .#default

# Build OCI images
nix build .#backend    # Backend container
nix build .#frontend   # Frontend (nginx) container

# Development shell
nix develop
```

## Deployment

### Development

Single binary serving both API and static files on port 3000.

```bash
cargo run
```

### Nix OCI Containers

Two-container deployment with nginx reverse proxy.

```bash
# Build images
nix build .#backend -o result-backend
nix build .#frontend -o result-frontend

# Load into container runtime
docker load < result-backend
docker load < result-frontend
```

### Incus / LXC

```bash
# Import images
lxc image import result-backend.tar.gz --alias omp-vis-backend
lxc image import result-frontend.tar.gz --alias omp-vis-frontend

# Launch
lxc launch omp-vis-backend backend
lxc launch omp-vis-frontend frontend

# Expose
lxc config device add frontend http proxy \
  connect="tcp:127.0.0.1:8080" listen="tcp:0.0.0.0:80"
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust (edition 2024) |
| Web Framework | Axum 0.8 |
| Templates | Tera (Jinja2-compatible) |
| Database | SQLite via rusqlite (FTS5 for search) |
| Serialization | serde + serde_json |
| Compression | flate2 (gzip) |
| Build System | Cargo + Nix Flakes |
| Frontend | Minelogue (AGPL-3.0, modified for omp) |
| Container | OCI via nixpkgs dockerTools |

## License

MIT — see [LICENSE](LICENSE) for details.

The frontend static assets (under `frontend/static/`) are derived from [minelogue](https://github.com/WeZZard/minelogue) and retain their AGPL-3.0 license.

## Links

- **GitHub**: [1hpEcVns/omp-visualizer](https://github.com/1hpEcVns/omp-visualizer)
- **Minelogue**: [WeZZard/minelogue](https://github.com/WeZZard/minelogue)
