#!/bin/bash
set -e

echo "🔧 Installing PostgreSQL..."
apt update -qq
apt install -y postgresql postgresql-contrib > /dev/null 2>&1

echo "🔧 Starting PostgreSQL..."
service postgresql start

echo "🔧 Creating database..."
su - postgres -c "psql" << 'EOF'
CREATE USER parser_user WITH PASSWORD 'secure_password_123';
CREATE DATABASE rights_parser OWNER parser_user;
GRANT ALL PRIVILEGES ON DATABASE rights_parser TO parser_user;
\q
EOF

echo "🔧 Configuring PostgreSQL..."
echo "host    rights_parser    parser_user    127.0.0.1/32    md5" >> /etc/postgresql/*/main/pg_hba.conf
service postgresql restart

echo "🔧 Starting Ollama..."
source "$HOME/.cargo/env"
ollama serve > /tmp/ollama.log 2>&1 &
sleep 10

echo "🎉 Setup complete!"
echo "✅ PostgreSQL: Running"
echo "✅ Ollama: Running"
echo "✅ GPU: $(nvidia-smi --query-gpu=name --format=csv,noheader)"
