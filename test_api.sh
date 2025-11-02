#!/bin/bash
# Quick API test script

BASE_URL="http://127.0.0.1:8080"

echo "=========================================="
echo "Testing Axiomind Web Server API"
echo "=========================================="
echo

echo "1. Health Check"
curl -i "$BASE_URL/health"
echo
echo

echo "2. Index Page (HEAD)"
curl -I "$BASE_URL/"
echo

echo "3. Static CSS (HEAD)"
curl -I "$BASE_URL/static/css/app.css"
echo

echo "4. Static JS (HEAD)"
curl -I "$BASE_URL/static/js/game.js"
echo

echo "5. Create Session"
curl -X POST "$BASE_URL/api/sessions" \
  -H "Content-Type: application/json" \
  -d '{"opponent":"baseline","starting_stack":1000,"small_blind":10,"big_blind":20}'
echo
echo

echo "6. Get Settings"
curl -i "$BASE_URL/api/settings"
echo
echo

echo "Done!"
