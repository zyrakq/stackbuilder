#!/bin/bash
# Base setup script for stackbuilder application

echo "Running base setup..."

# Initialize database
echo "Setting up base database configuration..."
export DB_HOST=${DB_HOST:-localhost}
export DB_PORT=${DB_PORT:-5432}
export DB_NAME=${DB_NAME:-app_db}

# Create basic directories
mkdir -p /app/logs
mkdir -p /app/data

echo "Base setup completed"