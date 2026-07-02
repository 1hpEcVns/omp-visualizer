#!/usr/bin/env bash
# OMP Session Visualizer - Deployment Script
# Usage: bash deploy/up.sh

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PORT="${PORT:-3000}"

echo "=== OMP Session Visualizer ==="
echo "Project: $PROJECT_DIR"
echo "Port: $PORT"
echo ""

# Option 1: Build and run with cargo (development)
run_dev() {
    echo "--- Development Mode ---"
    cd "$PROJECT_DIR/backend"
    cargo build --release 2>/dev/null || cargo build
    echo "Starting server on port $PORT..."
    RUST_LOG=info ./target/debug/omp-visualizer
}

# Option 2: Build with Nix
build_nix() {
    echo "--- Nix Build ---"
    cd "$PROJECT_DIR"
    nix build .#default
    echo "Binary: $(readlink -f result)/bin/omp-visualizer"
}

# Option 3: Build Nix OCI image
build_oci() {
    echo "--- OCI Image Build ---"
    cd "$PROJECT_DIR"
    nix build .#backend
    echo "OCI image: $(readlink -f result)"
}

# Option 4: Deploy with incus (if available)
deploy_incus() {
    echo "--- Incus Deployment ---"
    if ! command -v incus &> /dev/null && ! command -v lxc &> /dev/null; then
        echo "Incus/LXC not found. Install with: nix-shell -p incus"
        echo "Falling back to development mode..."
        run_dev
        return
    fi

    local CMD="incus"
    command -v lxc &> /dev/null && CMD="lxc"

    cd "$PROJECT_DIR"
    nix build .#backend -o result-backend
    
    echo "Importing OCI image..."
    $CMD image import result-backend --alias omp-vis-backend
    
    echo "Launching container..."
    $CMD launch omp-vis-backend omp-visualizer
    
    echo "Configuring network..."
    $CMD config device add omp-visualizer http proxy connect="tcp:127.0.0.1:$PORT" listen="tcp:0.0.0.0:3000"
    
    echo ""
    echo "=== Deployment Complete ==="
    echo "Access at: http://localhost:$PORT"
}

# Main
case "${1:-dev}" in
    dev)     run_dev ;;
    nix)     build_nix ;;
    oci)     build_oci ;;
    incus)   deploy_incus ;;
    *)
        echo "Usage: $0 {dev|nix|oci|incus}"
        echo "  dev   - Build and run with cargo (default)"
        echo "  nix   - Build with Nix flakes"
        echo "  oci   - Build OCI container image"
        echo "  incus - Deploy with incus/lxc"
        exit 1
        ;;
esac
