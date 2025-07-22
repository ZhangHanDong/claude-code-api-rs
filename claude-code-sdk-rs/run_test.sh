#!/bin/bash
echo "Starting test with debug logging..."
RUST_LOG=claude_code_sdk=debug cargo run --bin test_interactive 2>&1 &
PID=$!
sleep 10
echo -e "\n\nTimeout reached, killing process..."
kill $PID 2>/dev/null
wait $PID 2>/dev/null
echo "Test completed"