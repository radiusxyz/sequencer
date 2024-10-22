#!/bin/bash
CURRENT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

SEEDER_BIN_PATH=$CURRENT_PATH/../target/release/seeder
SEQUENCER_BIN_PATH=$CURRENT_PATH/../target/release/sequencer

HOST=$(ifconfig | grep 'inet ' | grep -v '127.0.0.1' | awk '{print $2}' | head -n 1)

SEQUENCER_HOST=$HOST
SEEDER_HOST=$HOST
LIVENESS_HOST=$HOST
VALIDATION_HOST=$HOST
KEY_MANAGEMENT_SYSTEM_HOST=$HOST

SEEDER_RPC_URL="http://$SEEDER_HOST:6000"

SEQUENCER_EXTERNAL_RPC_URL="http://$SEQUENCER_HOST:3000"
SEQUENCER_INTERNAL_RPC_URL="http://$SEQUENCER_HOST:4000"
SEQUENCER_CLUSTER_RPC_URL="http://$SEQUENCER_HOST:5000"

SEQUENCER_ADDRESS="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

# Sequencing info
PLATFORM="ethereum"
SERVICE_PROVIDER="radius"
LIVENESS_RPC_URL="http://$LIVENESS_HOST:8545"
LIVENESS_WS_URL="ws://$LIVENESS_HOST:8545"
EIGENLAYER_CONTRACT_ADDRESS="0x70e0bA845a1A0F2DA3359C97E0285013525FFC49"
SYMBIOTICS_CONTRACT_ADDRESS="0xf5059a5D33d5853360D16C683c16e67980206f36"

# Cluster info
CLUSTER_ID="cluster_id"

# Rollup
ROLLUP_ID="rollup_id"

# VAlidation info
VALIDATION_RPC_URL="http://$VALIDATION_HOST:8545"
VALIDATION_WS_URL="ws://$VALIDATION_HOST:8545"
DELIGATION_MANAGER_CONTRACT_ADDRESS="0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9"
STAKE_REGISTRY_CONTRACT_ADDRESS="0x9E545E3C0baAB3E08CdfD552C960A1050f373042"
AVS_DIRECTORY_CONTRACT_ADDRESS="0x5FC8d32690cc91D4c39d9d3abcBD16989F875707"
AVS_CONTRACT_ADDRESS="0x84eA74d481Ee0A5332c457a4d796187F6Ba67fEB"

# Symbiotics validation info
PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
OPERATOR_ADDRESS=0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266

NETWORK_OPTIN_SERVICE_CONTRACT_ADDRESS=0x8A791620dd6260079BF849Dc5567aDC3F2FdC318
OPERATOR_VAULT_OPT_IN_SERVICE_CONTRACT_ADDRESS=0x2279B7A0a67DB372996a5FaB50D91eAA73d2eBe6
VAULT_ADDRESS=0x7C7ceb9Ef4A4EbE265815EBf24E31905b6B86047
NETWORK_ADDRESS=0xf39fd6e51aad88f6f4ce6ab8827279cfffb9226