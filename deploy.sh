#!/usr/bin/env bash
set -Eeuo pipefail

exec 9>/tmp/svogame-auth-deploy.lock
flock -n 9 || {
  echo "Another deployment is already running"
  exit 1
}

git config --global --add safe.directory .
git fetch origin production
git reset --hard origin/production

docker compose pull
docker compose up -d --build --remove-orphans
