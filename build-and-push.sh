#!/bin/bash

REGISTRY="harbor.vect.one"
IMAGE_BASE="$REGISTRY/quantatrisk/cc-api"
TAG="${TAG:-latest}"

# Build and push for multiple platforms
docker buildx build \
  --no-cache \
  --platform linux/amd64,linux/arm64 \
  -t "$IMAGE_BASE:$TAG" \
  --push \
  .