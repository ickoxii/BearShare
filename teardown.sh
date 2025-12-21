#!/usr/bin/env bash

# stop the frontend
docker compose -f docker/local.docker-compose.yml down frontend -v

# stop the server
docker compose -f docker/ci.docker-compose.yml down server -v

# stop the database
docker compose -f docker/local.docker-compose.yml down db -v

# prune
docker image prune -af
docker system prune -f
