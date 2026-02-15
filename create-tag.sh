#!/bin/bash

# Script to create and push git tags based on Cargo.toml version
# Usage: ./create-tag.sh [--push]

set -e

# Get version from Cargo.toml
VERSION=$(grep -m 1 '^version = "' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

if [ -z "$VERSION" ]; then
    echo "❌ Could not find version in Cargo.toml"
    exit 1
fi

TAG="v$VERSION"

echo "📦 Version: $VERSION"
echo "🏷️  Tag: $TAG"

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "⚠️  Tag $TAG already exists"
    read -p "Do you want to delete and recreate it? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git tag -d "$TAG"
        echo "✅ Deleted local tag"
    else
        echo "❌ Aborted"
        exit 1
    fi
fi

# Create tag
echo "📝 Creating tag $TAG..."
git tag -a "$TAG" -m "Release $TAG

Changes:
- Version $VERSION
- Build binaries for Linux, macOS, and Windows
- Support for x86_64 and ARM64 architectures
"

if [ "$1" == "--push" ]; then
    echo "🚀 Pushing tag to origin..."
    git push origin "$TAG"
    echo "✅ Tag pushed successfully!"
    echo ""
    echo "GitHub Actions will now:"
    echo "1. Build binaries for all platforms"
    echo "2. Create a GitHub release"
    echo "3. Upload binaries to the release"
    echo ""
    echo "🔗 Check progress at: https://github.com/$(git remote get-url origin | sed 's/.*github.com[:/]\(.*\)\.git/\1/')/actions"
else
    echo "✅ Tag created successfully!"
    echo ""
    echo "To push the tag and trigger the release workflow:"
    echo "  git push origin $TAG"
    echo ""
    echo "Or use: ./create-tag.sh --push"
fi
