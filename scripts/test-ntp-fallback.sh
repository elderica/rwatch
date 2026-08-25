#!/bin/bash

# NTP Fallback Test Script
# Tests rwatch behavior when NTP servers are unreachable

set -e

RWATCH_BIN="$HOME/projects/rwatch/target/release/rwatch"
RWATCH_DIR="$HOME/projects/rwatch"
TEST_LOG="/tmp/rwatch-ntp-fallback-test.log"
TEST_DURATION=10

echo "=== NTP Fallback Test ==="
echo "Test duration: ${TEST_DURATION}s"
echo "Rwatch binary: $RWATCH_BIN"
echo ""

# Cleanup
rm -f "$TEST_LOG"

# Run rwatch in bwrap with no network access
echo "Starting rwatch in isolated network namespace (no NTP access)..."
(
    bwrap \
        --ro-bind "$RWATCH_DIR" "$RWATCH_DIR" \
        --proc /proc \
        --dev /dev \
        --tmpfs /tmp \
        --unshare-net \
        --die-with-parent \
        --chdir "$RWATCH_DIR" \
        "$RWATCH_BIN" \
        > "$TEST_LOG" 2>&1
) &
RWATCH_PID=$!

echo "Rwatch PID: $RWATCH_PID"
echo ""

# Wait and monitor
echo "Waiting ${TEST_DURATION}s to observe behavior..."
sleep $TEST_DURATION

# Check if process is still running
if kill -0 $RWATCH_PID 2>/dev/null; then
    echo ""
    echo "✓ Rwatch is still running after ${TEST_DURATION}s without NTP"
    echo ""
    
    # Show logs
    echo "=== Log Output ==="
    cat "$TEST_LOG"
    echo ""
    
    # Graceful shutdown
    echo "=== Graceful Shutdown Test ==="
    kill -TERM $RWATCH_PID
    sleep 2
    
    if ! kill -0 $RWATCH_PID 2>/dev/null; then
        echo "✓ Rwatch shut down gracefully"
    else
        echo "✗ Rwatch did not shut down"
        kill -9 $RWATCH_PID
    fi
    
    echo ""
    echo "=== Final Log Output ==="
    cat "$TEST_LOG"
    echo ""
    echo "=== TEST PASSED ==="
    exit 0
else
    echo ""
    echo "✗ Rwatch exited prematurely"
    echo ""
    echo "=== Log Output ==="
    cat "$TEST_LOG"
    echo ""
    echo "=== TEST FAILED ==="
    exit 1
fi
