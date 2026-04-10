#!/usr/bin/env bash
set -Eeuo pipefail

exec 9>/tmp/stage-svogame-auth-deploy.lock
flock -n 9 || {
  echo "Another deployment is already running"
  exit 1
}

git fetch origin stage
git reset --hard origin/stage

docker compose pull
docker compose up -d --build --remove-orphans
