#!/bin/bash
# debug-llm.sh - Test what the LLM is actually returning

echo "🧪 Testing LLM Output Directly"
echo "================================"
echo ""

# Test with simple input
echo "Test 1: Simple Agreement"
echo "------------------------"
echo "Parse this: Vyjayanthi Movies licenses Kalki 2898 AD to Zee Entertainment for India SVOD, INR 100, perpetuity, starring Prabhas and Deepika Padukone, directed by Nag Ashwin" | ollama run rights-parser

echo ""
echo ""
echo "Test 2: More Complex Input"
echo "--------------------------"
cat << 'EOF' | ollama run rights-parser
ASSIGNMENT AGREEMENT
Assignor: Vyjayanthi Movies
Assignee: Zee Entertainment Enterprises Limited
Film: Kalki 2898 AD
Territory: India
Media Rights: Linear TV, Cable, SVOD, Catch Up TV
Deal Value: INR 100
Term: January 1, 2025 to December 31, 2031 (7 years)
Exclusivity: Exclusive
Language: Telugu (dubbed to Hindi, Tamil, Kannada, Malayalam)
Lead Cast: Prabhas, Deepika Padukone, Amitabh Bachchan, Kamal Haasan
Director: Nag Ashwin
Release Date: June 27, 2024
Duration: 180 minutes
EOF

echo ""
echo "================================"
echo "✅ If you see clean JSON above (starting with { and ending with }), the model works!"
echo "❌ If you see text/explanations, the model needs more training."