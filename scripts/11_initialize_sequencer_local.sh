#!/bin/bash
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

echo "add_sequencing_info"
curl --location $SEQUENCER_INTERNAL_RPC_URL \
--header 'Content-Type: application/json' \
--data '{
  "jsonrpc": "2.0",
  "method": "add_sequencing_info",
  "params": {
    "platform": "'"$PLATFORM"'",
    "service_provider": "'"$SERVICE_PROVIDER"'",
    "payload": {
      "liveness_rpc_url": "'"$LIVENESS_RPC_URL"'",
      "liveness_websocket_url": "'"$LIVENESS_WS_URL"'",
      "contract_address": "'"$CONTRACT_ADDRESS"'"
    }
  },
  "id": 1
}'
echo ""
echo "add_sequencing_info done"
sleep 0.5

echo "add_cluster"
curl --location $SEQUENCER_INTERNAL_RPC_URL \
--header 'Content-Type: application/json' \
--data '{
  "jsonrpc": "2.0",
  "method": "add_cluster",
  "params": {
    "platform": "'"$PLATFORM"'",
    "service_provider": "'"$SERVICE_PROVIDER"'",
    
    "cluster_id": "'"$CLUSTER_ID"'"
  },
  "id": 1
}'
echo "add_cluster done"
sleep 0.5

echo "add_validation_info"
curl --location $SEQUENCER_INTERNAL_RPC_URL \
--header 'Content-Type: application/json' \
--data '{
  "jsonrpc": "2.0",
  "method": "add_validation_info",
  "params": {
    "platform": "'"$PLATFORM"'",
    "service_provider": "'"$SERVICE_PROVIDER"'",
    "payload": {
      "validation_rpc_url": "'"$VALIDATION_RPC_URL"'",
      "validation_websocket_url": "'"$VALIDATION_WS_URL"'",
      "delegation_manager_contract_address": "'"$DELIGATION_MANAGER_CONTRACT_ADDRESS"'",
      "stake_registry_contract_address": "'"$STAKE_REGISTRY_CONTRACT_ADDRESS"'",
      "avs_directory_contract_address": "'"$AVS_DIRECTORY_CONTRACT_ADDRESS"'",
      "avs_contract_address": "'"$AVS_CONTRACT_ADDRESS"'"
    }
  },
  "id": 1
}'
echo ""
echo "add_validation_info done"