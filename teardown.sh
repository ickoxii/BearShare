#!/usr/bin/env bash

teardown() {
    local ENV=local

    # stop the frontend
    docker compose -f docker/${ENV}/docker-compose.yml down frontend -v

    # stop the server
    docker compose -f docker/${ENV}/docker-compose.yml down server -v

    # stop the database
    docker compose -f docker/${ENV}/docker-compose.yml down db -v
}

teardown

# prune
docker image prune -af
docker system prune -f
