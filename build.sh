#!/usr/bin/env bash

set -euo pipefail

build() {
  local ENV="local"

  local DB=""
  local SERVER=""
  local FRONTEND=""

  if [[ $# -eq 0 ]]; then
    DB="db"
    SERVER="server"
    FRONTEND="frontend"
  else
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
  fi

  docker compose --parallel 3 -f docker/${ENV}/docker-compose.yml up ${DB} ${SERVER} ${FRONTEND} -d --build
}

build $@
