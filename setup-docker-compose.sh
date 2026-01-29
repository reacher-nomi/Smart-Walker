#!/bin/bash
# One-time setup to make "docker-compose up" work automatically
# Run this once: bash setup-docker-compose.sh

echo "🔧 Setting up docker-compose wrapper..."

# Make wrapper executable
chmod +x docker-compose

# Add to PATH for current session
export PATH="$(pwd):$PATH"

# Add to .bashrc for future sessions
if ! grep -q "Smart-Walker" ~/.bashrc 2>/dev/null; then
    echo "" >> ~/.bashrc
    echo "# Smart-Walker docker-compose wrapper" >> ~/.bashrc
    echo "export PATH=\"$(pwd):\$PATH\"" >> ~/.bashrc
    echo "✅ Added to ~/.bashrc"
fi

echo "✅ Setup complete!"
echo ""
echo "Now you can use: docker-compose up -d"
echo "The .env file will be created automatically if it doesn't exist."
