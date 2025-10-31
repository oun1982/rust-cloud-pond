#!/bin/bash
# Test script to simulate Asterisk AGI environment

echo "Testing AGI mode..."
echo ""

# Simulate AGI environment being sent to the script
{
    echo "agi_request: check-login"
    echo "agi_channel: PJSIP/4001-00000012"
    echo "agi_language: en"
    echo "agi_type: PJSIP"
    echo "agi_uniqueid: 1234567890.123"
    echo "agi_version: 18.0.0"
    echo "agi_callerid: 4001"
    echo "agi_calleridname: Test User"
    echo "agi_callingpres: 0"
    echo "agi_callingani2: 0"
    echo "agi_callington: 0"
    echo "agi_dnid: 4001"
    echo "agi_rdnis: unknown"
    echo "agi_context: from-internal"
    echo "agi_extension: s"
    echo "agi_priority: 2"
    echo "agi_enhanced: 0.0"
    echo "agi_accountcode: "
    echo "agi_threadid: 139876543210000"
    echo ""  # Empty line marks end of AGI environment
} | AGI_REQUEST="agi_request: check-login" AGI_CHANNEL="PJSIP/4001-00000012" /opt/rust-project/rust-login/target/release/rust-login "$@"

echo ""
echo "Exit code: $?"
