#!/usr/bin/env bash

build() {
    local ENV=local

    # start the database
    docker compose -f docker/${ENV}/docker-compose.yml up db -d --build

    # start the server
    docker compose -f docker/${ENV}/docker-compose.yml up server -d --build

    # start the frontend
    docker compose -f docker/${ENV}/docker-compose.yml up frontend -d --build
}

build
