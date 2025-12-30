#!/bin/bash
API_KEY="VbwPP5fFpw/16Sm6GygYhc29oLyqMgcUbyAKWUiEf3c="

echo "1️⃣ Testing Health..."
curl -s http://localhost:8080/health | jq .

echo -e "\n2️⃣ Uploading PDF with queue_id and callback_url..."
RESPONSE=$(curl -s -X POST http://localhost:8080/api/v1/upload \
  -H "X-API-Key: $API_KEY" \
  -F "file=@./Kalki.pdf" \
  -F "queue_id=1" \
  -F "callback_url=https://digitalrights.tfhy.in/api/rights-management/ai-contract-callback/")

echo $RESPONSE | jq .

JOB_ID=$(echo $RESPONSE | jq -r '.job_id')
echo -e "\n✅ Job ID: $JOB_ID"

echo -e "\n3️⃣ Checking Status..."
curl -s http://localhost:8080/api/v1/status/$JOB_ID | jq .

echo -e "\n⏳ Waiting 100 seconds for processing..."
sleep 100

echo -e "\n4️⃣ Final Status..."
curl -s http://localhost:8080/api/v1/status/$JOB_ID | jq .

echo -e "\n5️⃣ Getting Result..."
curl -s http://localhost:8080/api/v1/result/$JOB_ID | jq .
