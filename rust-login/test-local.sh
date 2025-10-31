#!/bin/bash
# Test script for debugging

echo "=== Testing with CORRECT credentials (4001) ==="
echo ""
./target/release/rust-login 10.133.1.11 login 4001 4001 4001
echo "Exit code: $?"
echo ""
echo "=== Testing with WRONG credentials (4055) ==="
echo ""
./target/release/rust-login 10.133.1.11 login 4055 4055 4055
echo "Exit code: $?"
