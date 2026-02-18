#!/bin/bash
# run-unit-tests-local.sh - Run unit tests with Docker container
#
# Usage:
#   ./run-unit-tests-local.sh                     # run all tests
#   ./run-unit-tests-local.sh cgroup              # run tests matching "cgroup"
#   ./run-unit-tests-local.sh namespace           # run tests matching "namespace"
#   ./run-unit-tests-local.sh --privileged        # run all tests with privileged mode (for cgroup tests)
#   ./run-unit-tests-local.sh --no-cache          # force rebuild of Docker image

set -e

IMAGE_NAME="rundmc-test"
PRIVILEGED=""
NO_CACHE=""
TEST_FILTER=""

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        --privileged)
            PRIVILEGED="--privileged"
            ;;
        --no-cache)
            NO_CACHE="--no-cache"
            ;;
        --*)
            echo "Unknown option: $arg"
            exit 1
            ;;
        *)
            TEST_FILTER="$arg"
            ;;
    esac
done

# Docker availability check
if ! docker info > /dev/null 2>&1; then
    echo "Error: Docker is not running. Please start Docker Desktop first."
    exit 1
fi

echo "=== rundmc unit tests (Linux container) ==="
echo ""

# Build Docker image
echo "Building test image..."
docker build $NO_CACHE -f Dockerfile.test -t $IMAGE_NAME . --quiet
echo "Image ready: $IMAGE_NAME"
echo ""

# Build the cargo test command
if [ -n "$TEST_FILTER" ]; then
    CARGO_CMD="cargo test $TEST_FILTER -- --nocapture"
    echo "Running tests matching: '$TEST_FILTER'"
else
    CARGO_CMD="cargo test -- --nocapture"
    echo "Running all tests"
fi

if [ -n "$PRIVILEGED" ]; then
    echo "Mode: privileged (cgroup access enabled)"
else
    echo "Mode: normal (use --privileged to enable cgroup v2 access)"
fi

echo ""
echo "---"

# Run tests
docker run --rm \
    $PRIVILEGED \
    -e RUST_LOG="${RUST_LOG:-debug}" \
    $IMAGE_NAME \
    sh -c "$CARGO_CMD"

echo "---"
echo ""
echo "Done."
