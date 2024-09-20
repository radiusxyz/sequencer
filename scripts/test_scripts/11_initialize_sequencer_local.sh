#!/bin/bash
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
source $SCRIPT_PATH/env.sh

echo "add_sequencing_info"
curl --location ${INTERNAL_RPC_URLS[0]} \
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
curl --location ${INTERNAL_RPC_URLS[0]} \
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