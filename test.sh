#!/bin/bash
# test.sh - Test the Rights Parser API

set -e

echo "🧪 Testing Rights Parser API"
echo "============================"
echo ""

API_URL="http://localhost:8080"

# Test 1: Health Check
echo "Test 1: Health Check"
curl -s $API_URL/api/health | jq .
echo ""

# Test 2: Create sample PDF text file
echo "Test 2: Creating sample agreement text..."
cat > /tmp/sample-agreement.txt << 'EOF'
DISTRIBUTION LICENSE AGREEMENT

This agreement made on December 10, 2024

BETWEEN:
Sony Pictures Entertainment Inc. ("Licensor")
10202 West Washington Boulevard
Culver City, CA 90232, USA

AND:
Zee Entertainment Enterprises Limited ("Licensee")
18th Floor, A Wing, Marathon Futurex
N. M. Joshi Marg, Lower Parel
Mumbai-400013, India

CONTENT: Spider-Man: Across the Spider-Verse
Type: Animated Feature Film
Language: English
Genre: Animation, Action, Adventure, Superhero
Duration: 140 minutes
Director: Joaquim Dos Santos, Kemp Powers, Justin K. Thompson
Producer: Avi Arad, Amy Pascal, Phil Lord, Christopher Miller
Release Date: June 2, 2023
Rating: U/A (CBFC), PG (MPAA)

TERRITORY: India, Pakistan, Bangladesh, Sri Lanka, Nepal, Bhutan, Maldives

MEDIA RIGHTS:
- Subscription Video on Demand (SVOD)
- Ad-supported Video on Demand (AVOD)
- Linear Television
- Cable Television
- Satellite Broadcasting
- OTT Platforms
- Mobile Streaming

TERM: Five (5) years
Commencement: January 1, 2025
Expiration: January 1, 2030

EXCLUSIVITY: Exclusive rights in the specified territories

LICENSE FEE: USD 2,500,000 (Two Million Five Hundred Thousand US Dollars)

PAYMENT STRUCTURE:
- Upon Signing: USD 1,000,000 (40%)
- Upon Content Delivery: USD 1,500,000 (60%)

DELIVERABLES:
Video Formats: 4K UHD, HD 1080p, HD 720p, SD
Audio: Dolby Atmos, 5.1 Surround, Stereo
Subtitles: Hindi, Tamil, Telugu, Bengali, Marathi, Gujarati, Kannada, Malayalam
Dubbing: Hindi, Tamil, Telugu

TECHNICAL SPECIFICATIONS:
Video Codec: H.265/HEVC
Audio Codec: AAC, Dolby Digital Plus
Container: MP4, MKV
DRM: Widevine, PlayReady, FairPlay

GOVERNING LAW: Laws of India
DISPUTE RESOLUTION: Arbitration in Mumbai

Signed on behalf of Sony Pictures Entertainment Inc.
Tom Rothman
Chairman and CEO

Signed on behalf of Zee Entertainment Enterprises Limited
Punit Goenka
Managing Director & CEO
EOF

echo "✅ Sample agreement created"
echo ""

# Convert to PDF (if you have wkhtmltopdf installed)
if command -v wkhtmltopdf &> /dev/null; then
    echo "Test 3: Converting to PDF..."
    wkhtmltopdf /tmp/sample-agreement.txt /tmp/sample-agreement.pdf
    echo "✅ PDF created"
    echo ""
    
    # Test 4: Parse PDF
    echo "Test 4: Parsing PDF agreement..."
    curl -X POST $API_URL/api/parse \
      -F "pdf=@/tmp/sample-agreement.pdf" \
      -o /tmp/parsed-agreement.json
    
    echo "✅ PDF parsed successfully!"
    echo ""
    echo "Result saved to: /tmp/parsed-agreement.json"
    echo ""
    echo "Parsed Agreement:"
    cat /tmp/parsed-agreement.json | jq .
else
    echo "⚠️  wkhtmltopdf not installed, skipping PDF test"
    echo "   Install with: brew install wkhtmltopdf (macOS)"
    echo "   Or: apt-get install wkhtmltopdf (Ubuntu)"
fi

echo ""
echo "✅ All tests complete!"
echo ""
echo "To deploy to blockchain:"
echo "  cd rights-agreement-demo"
echo "  cp /tmp/parsed-agreement.json data/zee-eagle-agreement.json"
echo "  npm run deploy:full"
echo ""