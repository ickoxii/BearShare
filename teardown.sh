#!/usr/bin/env bash

set -euo pipefail

teardown() {
    local ENV=local

    local DB=""
    local SERVER=""
    local FRONTEND=""

    while [[ $# -gt 0 ]]; do
        case $1 in
            db)
                DB=db
                ;;
            server)
                SERVER=server
                ;;
            frontend)
                FRONTEND=frontend
                ;;
            local)
                ENV=local
                ;;
            prod)
                ENV=ci
                ;;
            *)
                echo "invalid command: $1"
                exit 1
        esac
        shift
    done

    docker compose -f docker/${ENV}/docker-compose.yml down ${DB} ${SERVER} ${FRONTEND} -v
}

teardown $@

# prune
docker image prune -af
docker system prune -f
