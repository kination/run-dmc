#!/bin/bash
# test-rundmc.sh - Detailed runtime testing with full logging

set -e

# Pre-flight checks
echo "🔍 Pre-flight checks..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Error: Docker is not running"
    echo "Please start Docker Desktop first"
    exit 1
fi

# Check if rundmc is registered
if ! docker info --format '{{json .Runtimes}}' 2>/dev/null | grep -q '"rundmc"'; then
    echo "❌ Error: rundmc runtime is not registered in Docker"
    echo ""
    echo "Please run the installation script first:"
    echo "  ./install-rundmc.sh"
    exit 1
fi

# Check if rundmc binary exists in VM
if ! docker exec vm-access nsenter -t 1 -m -u -n -i test -f /usr/local/bin/rundmc 2>/dev/null; then
    echo "❌ Error: rundmc binary not found in Docker VM"
    echo ""
    echo "Please run the installation script:"
    echo "  ./install-rundmc.sh"
    exit 1
fi

echo "✅ All checks passed"
echo ""

echo "=== Testing rundmc with detailed logging ==="
echo ""

# Test 1: Simple echo with DEBUG logging
echo "📝 Test 1: Simple echo command with DEBUG logging"
RUST_LOG=debug docker run --rm --runtime=rundmc --network=none alpine echo "Hello from rundmc" 2>&1 | grep -E "(rundmc|Creating|created|started|deleted)"
echo ""

# Test 2: Multi-command script (shows longer lifecycle)
echo "📝 Test 2: Multi-step script (sleep + multiple commands)"
RUST_LOG=info docker run --rm --runtime=rundmc --network=none alpine sh -c '
    echo "Step 1: Container started"
    sleep 1
    echo "Step 2: After 1 second"
    sleep 1
    echo "Step 3: After 2 seconds"
    echo "Step 4: Exiting..."
' 2>&1 | tee /tmp/rundmc-test.log
echo ""

# Test 3: Check state during execution (requires background container)
echo "📝 Test 3: State inspection test"
echo "Starting container in background..."
CONTAINER_ID=$(docker run -d --runtime=rundmc --network=none alpine sleep 5)
echo "Container ID: $CONTAINER_ID"

sleep 1
echo "Checking container state..."
docker inspect $CONTAINER_ID | grep -E "(Status|Pid|Runtime)" || true

sleep 2
echo "Cleaning up..."
docker rm -f $CONTAINER_ID
echo ""

# Test 4: Full lifecycle with timestamps
echo "📝 Test 4: Full lifecycle with timestamps"
echo "Start time: $(date '+%H:%M:%S.%3N')"
RUST_LOG=debug docker run --rm --runtime=rundmc --network=none alpine sh -c 'echo "Running at: $(date)"; sleep 2; echo "Done"' 2>&1 | while IFS= read -r line; do
    echo "[$(date '+%H:%M:%S.%3N')] $line"
done | grep -E "(rundmc|Creating|created|started|Running|Done|deleted)"
echo "End time: $(date '+%H:%M:%S.%3N')"
echo ""

echo "=== All tests completed ==="
echo "Full logs saved to: /tmp/rundmc-test.log"
