#!/bin/bash
set -e

echo "════════════════════════════════════════════════════════════"
echo "🚀 RunPod Rights Parser - Complete Setup Script"
echo "════════════════════════════════════════════════════════════"
echo ""

# ============================================
# 1. INSTALL SYSTEM DEPENDENCIES
# ============================================
echo "📦 Step 1/8: Installing system dependencies..."
apt update -qq
apt install -y \
    curl \
    build-essential \
    postgresql \
    postgresql-contrib \
    pkg-config \
    libssl-dev \
    libpq-dev \
    clang \
    libclang-dev \
    llvm-dev \
    jq \
    > /dev/null 2>&1
echo "✅ System dependencies installed"
echo ""

# ============================================
# 2. INSTALL RUST
# ============================================
echo "🦀 Step 2/8: Installing Rust..."
if [ ! -f "$HOME/.cargo/env" ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y > /dev/null 2>&1
fi
source "$HOME/.cargo/env"
echo "✅ Rust installed: $(cargo --version)"
echo ""

# ============================================
# 3. INSTALL OLLAMA
# ============================================
echo "🤖 Step 3/8: Installing Ollama..."
if ! command -v ollama &> /dev/null; then
    curl -fsSL https://ollama.com/install.sh | sh > /dev/null 2>&1
fi
echo "✅ Ollama installed"
echo ""

# ============================================
# 4. SETUP POSTGRESQL
# ============================================
echo "🗄️  Step 4/8: Setting up PostgreSQL..."
service postgresql start > /dev/null 2>&1

# Create database and user
su - postgres -c "psql" << 'EOF' > /dev/null 2>&1
CREATE USER parser_user WITH PASSWORD 'secure_password_123';
CREATE DATABASE rights_parser OWNER parser_user;
GRANT ALL PRIVILEGES ON DATABASE rights_parser TO parser_user;
\q
EOF

# Configure authentication
echo "host    rights_parser    parser_user    127.0.0.1/32    md5" >> /etc/postgresql/*/main/pg_hba.conf
service postgresql restart > /dev/null 2>&1

# Test connection
PGPASSWORD=secure_password_123 psql -h localhost -U parser_user -d rights_parser -c "SELECT 1;" > /dev/null 2>&1
echo "✅ PostgreSQL configured and running"
echo ""

# ============================================
# 5. APPLY DATABASE SCHEMA
# ============================================
echo "📊 Step 5/8: Applying database schema..."
cd /workspace/rights-parser-rust

# Apply main schema
PGPASSWORD=secure_password_123 psql -h localhost -U parser_user -d rights_parser < migrations/001_init.sql > /dev/null 2>&1

# Add missing columns
PGPASSWORD=secure_password_123 psql -h localhost -U parser_user -d rights_parser << 'EOF' > /dev/null 2>&1
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS queue_id TEXT;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS callback_url TEXT;
CREATE INDEX IF NOT EXISTS idx_jobs_queue_id ON jobs(queue_id);
EOF

# Add API key
API_KEY="VbwPP5fFpw/16Sm6GygYhc29oLyqMgcUbyAKWUiEf3c="
API_KEY_HASH=$(echo -n "$API_KEY" | sha256sum | awk '{print $1}')
PGPASSWORD=secure_password_123 psql -h localhost -U parser_user -d rights_parser << EOF > /dev/null 2>&1
INSERT INTO api_keys (key_hash, key_prefix, name, user_id, is_active)
VALUES ('$API_KEY_HASH', 'VbwP', 'Production Key', 'user_1', true)
ON CONFLICT DO NOTHING;
EOF

echo "✅ Database schema applied and API key added"
echo ""

# ============================================
# 6. START OLLAMA AND PULL MODELS
# ============================================
echo "🤖 Step 6/8: Starting Ollama and pulling models..."
source "$HOME/.cargo/env"
ollama serve > /tmp/ollama.log 2>&1 &
sleep 10

# Verify GPU detection
GPU_INFO=$(tail -50 /tmp/ollama.log | grep -i "Tesla T4" || echo "CPU mode")
if [[ $GPU_INFO == *"Tesla T4"* ]]; then
    echo "✅ GPU detected: Tesla T4"
else
    echo "⚠️  Running in CPU mode (slower)"
fi

# Pull base model
echo "   Pulling llama3.1:8b (this may take a few minutes)..."
ollama pull llama3.1:8b > /dev/null 2>&1

# Create custom model
echo "   Creating custom model llama31-parser..."
ollama create llama31-parser -f Modelfile > /dev/null 2>&1

echo "✅ Ollama running with models ready"
echo ""

# ============================================
# 7. BUILD RUST PROJECT
# ============================================
echo "🔨 Step 7/8: Building Rust project (this may take 5-10 minutes)..."
mkdir -p ./uploads
source "$HOME/.cargo/env"
cargo build --release

if [ -f "./target/release/rights-agreement-parser" ]; then
    echo "✅ Project built successfully"
else
    echo "❌ Build failed - check errors above"
    exit 1
fi
echo ""

# ============================================
# 8. CREATE ENVIRONMENT FILE
# ============================================
echo "⚙️  Step 8/8: Creating environment configuration..."
cat > .env << 'EOFENV'
DATABASE_URL=postgresql://parser_user:secure_password_123@localhost:5432/rights_parser
PORT=8080
OLLAMA_URL=http://localhost:11434
OLLAMA_MODEL=llama31-parser
PINATA_JWT=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJ1c2VySW5mb3JtYXRpb24iOnsiaWQiOiI0YWY4MzlhOS0wMDA0LTQzOWUtOTJhMi01MTFlMzgzNTY3MmIiLCJlbWFpbCI6InNyaWthci5yYW9AdGhlLWFsbHkuY29tIiwiZW1haWxfdmVyaWZpZWQiOnRydWUsInBpbl9wb2xpY3kiOnsicmVnaW9ucyI6W3siZGVzaXJlZFJlcGxpY2F0aW9uQ291bnQiOjEsImlkIjoiRlJBMSJ9LHsiZGVzaXJlZFJlcGxpY2F0aW9uQ291bnQiOjEsImlkIjoiTllDMSJ9XSwidmVyc2lvbiI6MX0sIm1mYV9lbmFibGVkIjpmYWxzZSwic3RhdHVzIjoiQUNUSVZFIn0sImF1dGhlbnRpY2F0aW9uVHlwZSI6InNjb3BlZEtleSIsInNjb3BlZEtleUtleSI6ImQyZGM5OGQ5NWM0MTVkOGQ5N2MzIiwic2NvcGVkS2V5U2VjcmV0IjoiMWNmZjNmYWEzNDY4NDBhOGViZTQ3NmExYmE3N2Y3YTZlZTk4ZWNkMjU4NDc3MmE2ZjBiMjY2Mjg5OTg3MDgxYSIsImV4cCI6MTc5ODA4MzAzOX0.18L5zVKODm0X9KsS78GnEXmLQFtQb_KLUfx6pt0chCM
UPLOAD_DIR=./uploads
RUST_LOG=info,rights_agreement_parser=debug,sqlx=warn
EOFENV
echo "✅ Environment file created"
echo ""

# ============================================
# SUMMARY
# ============================================
echo "════════════════════════════════════════════════════════════"
echo "🎉 SETUP COMPLETE!"
echo "════════════════════════════════════════════════════════════"
echo ""
echo "📊 System Status:"
echo "   ✅ PostgreSQL: Running on localhost:5432"
echo "   ✅ Ollama: Running on localhost:11434"
echo "   ✅ GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null || echo 'CPU mode')"
echo "   ✅ Models: llama3.1:8b, llama31-parser"
echo "   ✅ API Binary: ./target/release/rights-agreement-parser"
echo ""
echo "🚀 To start the API server:"
echo "   source .env && ./target/release/rights-agreement-parser"
echo ""
echo "🧪 To test:"
echo "   ./test_with_real_queue.sh"
echo ""
echo "📊 To monitor GPU:"
echo "   watch -n 2 nvidia-smi"
echo ""
echo "════════════════════════════════════════════════════════════"


chmod +x setup.sh
./setup.sh