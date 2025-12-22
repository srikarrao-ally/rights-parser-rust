#!/bin/bash
# setup.sh - Quick setup script for Rights Parser

set -e

echo "🚀 Rights Agreement Parser - Quick Setup"
echo "========================================"
echo ""

# Check if Ollama is installed
if ! command -v ollama &> /dev/null; then
    echo "📦 Installing Ollama..."
    curl -fsSL https://ollama.com/install.sh | sh
else
    echo "✅ Ollama already installed"
fi

# Start Ollama service
echo "🔧 Starting Ollama service..."
ollama serve &
sleep 5

# Pull base model
echo "📥 Pulling Llama 3 base model..."
ollama pull llama3

# Create fine-tuned model
echo "🎓 Creating fine-tuned rights-parser model..."
ollama create rights-parser -f Modelfile

# Test the model
echo "🧪 Testing model..."
echo "Agreement: Sony licenses Spider-Man to Zee for India SVOD rights, USD 2.5M, 5 years" | ollama run rights-parser

echo ""
echo "✅ Setup complete!"
echo ""
echo "To start the service:"
echo "  cargo run --release"
echo ""
echo "Or with Docker:"
echo "  docker-compose up -d"
echo ""
echo "API will be available at: http://localhost:8080"
echo ""