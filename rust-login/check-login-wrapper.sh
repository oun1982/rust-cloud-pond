#!/bin/bash
# AGI Wrapper for rust-login
# This script reads AGI environment and passes it to the Rust binary

# Export AGI_REQUEST so the Rust binary can detect AGI mode
export AGI_REQUEST="agi_request: check-login"
export AGI_CHANNEL="channel"

# Read and discard AGI environment from stdin
while IFS= read -r line; do
    # Empty line marks end of AGI environment
    if [ -z "$line" ]; then
        break
    fi
done

# Call the Rust binary with all arguments
# The binary will detect AGI mode via AGI_REQUEST environment variable
exec /var/lib/asterisk/agi-bin/rust-agi "$@"
