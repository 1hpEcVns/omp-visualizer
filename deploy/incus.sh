#!/usr/bin/env bash
# OMP Session Visualizer - Incus Deployment Script
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
echo "=== OMP Visualizer - Incus Deployment ==="

# Step 1: Build OCI image with Nix
echo "[1/5] Building OCI image with Nix..."
cd "$PROJECT_DIR"
nix build .#backend -o result-backend 2>&1 | tail -1
echo "OCI image: $(readlink -f result-backend)"

# Step 2: Import into incus
echo "[2/5] Importing into incus..."
incus image import result-backend --alias omp-vis-backend 2>/dev/null || \
  echo "Note: Direct OCI import may need format conversion. Using binary deployment instead."

# Step 3: Launch incus container (using Alpine for small footprint)
echo "[3/5] Launching container..."
incus launch images:alpine/edge omp-visualizer 2>/dev/null || \
  incus start omp-visualizer 2>/dev/null || true

# Step 4: Deploy binary and assets
echo "[4/5] Deploying binary..."
cargo build --release --manifest-path "$PROJECT_DIR/backend/Cargo.toml" 2>/dev/null || \
  cargo build --manifest-path "$PROJECT_DIR/backend/Cargo.toml"

BIN="$PROJECT_DIR/backend/target/debug/omp-visualizer"
incus file push "$BIN" omp-visualizer/usr/local/bin/omp-visualizer
incus exec omp-visualizer -- chmod +x /usr/local/bin/omp-visualizer

# Push static assets
incus exec omp-visualizer -- mkdir -p /frontend/static /backend/templates
incus file push "$PROJECT_DIR/frontend/static/" omp-visualizer/frontend/static/ --recursive
incus file push "$PROJECT_DIR/backend/templates/" omp-visualizer/backend/templates/ --recursive

# Step 5: Configure and start
echo "[5/5] Starting service..."
# Proxy port 3000 from host to container
incus config device add omp-visualizer http proxy connect="tcp:127.0.0.1:3000" listen="tcp:0.0.0.0:3000" 2>/dev/null || true

# Start the binary in background
incus exec omp-visualizer -- sh -c 'HOME=/root nohup /usr/local/bin/omp-visualizer > /tmp/omp.log 2>&1 &'

echo ""
echo "=== Deployment Complete ==="
echo "Dashboard: http://localhost:3000/"
echo "API:       http://localhost:3000/api/health"
echo "Logs:      incus exec omp-visualizer -- cat /tmp/omp.log"
echo ""
echo "Manual verification:"
echo "  curl http://localhost:3000/api/health"
echo "  curl http://localhost:3000/api/sessions?agent=omp"
