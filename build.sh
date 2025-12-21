#!/usr/bin/env bash

# start the database
docker compose -f docker/local.docker-compose.yml up db -d --build

# start the server
docker compose -f docker/ci.docker-compose.yml up server -d --build

# start the frontend
docker compose -f docker/local.docker-compose.yml up frontend -d --build
