#!/usr/bin/env bash
set -euo pipefail

# BLEEP Interchain Transaction Demo
# This script demonstrates how to perform an interchain transaction between BLEEP and Ethereum

echo "🚀 BLEEP Interchain Transaction Demo"
echo "===================================="

# Check if node is running
echo "📡 Checking BLEEP node status..."
if ! curl -s --connect-timeout 5 http://127.0.0.1:8545/rpc/health >/dev/null; then
    echo "❌ BLEEP node not running. Please start it first:"
    echo "   SEPOLIA_BLEEP_FULFILL_ADDR=0x1234567890abcdef1234567890abcdef12345678 ./target/release/bleep"
    exit 1
fi
echo "✅ BLEEP node is running"

# Check environment variable
if [[ -z "${SEPOLIA_BLEEP_FULFILL_ADDR:-}" ]]; then
    echo "⚠️  SEPOLIA_BLEEP_FULFILL_ADDR not set. Setting to test value..."
    export SEPOLIA_BLEEP_FULFILL_ADDR=0x1234567890abcdef1234567890abcdef12345678
fi
echo "✅ Sepolia contract address: $SEPOLIA_BLEEP_FULFILL_ADDR"

echo ""
echo "📝 Step 1: Submit an interchain intent"
echo "Transferring 1 BLEEP to Ethereum address 0x742d35Cc6634C0532925a3b844Bc454e4438f44e"

INTENT_RESPONSE=$(curl -s -X POST http://127.0.0.1:8545/rpc/connect/intent \
  -H "Content-Type: application/json" \
  -d '{
    "source_chain": "BLEEP",
    "dest_chain": "Ethereum",
    "source_amount": 1000000000000000000,
    "min_dest_amount": 900000000000000000,
    "sender_address": "BLEEP1test1234567890abcdef1234567890abcdef12345678",
    "recipient_address": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "max_solver_reward_bps": 50,
    "slippage_tolerance_bps": 100
  }')

echo "Intent submission response:"
echo "$INTENT_RESPONSE" | jq .

INTENT_ID=$(echo "$INTENT_RESPONSE" | jq -r '.intent_id')
if [[ "$INTENT_ID" == "null" || -z "$INTENT_ID" ]]; then
    echo "❌ Failed to submit intent"
    exit 1
fi

echo ""
echo "✅ Intent submitted successfully!"
echo "Intent ID: $INTENT_ID"

echo ""
echo "📊 Step 2: Check intent status"
sleep 2
STATUS_RESPONSE=$(curl -s http://127.0.0.1:8545/rpc/connect/intent/$INTENT_ID)
echo "Intent status:"
echo "$STATUS_RESPONSE" | jq .

echo ""
echo "🔗 Step 3: Build Sepolia relay transaction"
RELAY_RESPONSE=$(curl -s http://127.0.0.1:8545/rpc/connect/intent/$INTENT_ID/relay_tx)
echo "Relay transaction:"
echo "$RELAY_RESPONSE" | jq .

echo ""
echo "🎯 Step 4: Simulate the relay (for demonstration)"
echo "In a real scenario, you would:"
echo "1. Submit the relay transaction to Ethereum Sepolia"
echo "2. Wait for confirmation"
echo "3. The BleepFulfill contract would receive the ETH"
echo "4. The recipient would get their funds"

echo ""
echo "📋 Summary:"
echo "- Intent ID: $INTENT_ID"
echo "- Source: 1 BLEEP on BLEEP chain"
echo "- Destination: ~0.9 ETH on Ethereum Sepolia"
echo "- Contract: $SEPOLIA_BLEEP_FULFILL_ADDR"
echo "- Status: Ready for relay execution"

echo ""
echo "✨ Interchain transaction demonstration complete!"