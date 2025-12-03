#!/bin/bash
# Quick commit script for TempRS
# Usage: ./commit.sh "Your commit message"

if [ -z "$1" ]; then
    echo "❌ Error: Commit message required"
    echo "Usage: ./commit.sh \"Your commit message\""
    exit 1
fi

echo "📝 Staging all changes..."
git add -A

echo "📊 Changes to commit:"
git status --short

echo ""
read -p "Continue with commit? (y/n) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "💾 Committing..."
    git commit -m "$1"
    echo "✅ Done!"
    echo ""
    echo "Recent commits:"
    git log --oneline -3
else
    echo "❌ Commit cancelled"
    git reset HEAD
fi
