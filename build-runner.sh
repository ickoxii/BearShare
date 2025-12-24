#!/usr/bin/env bash

set -e

IMAGE_NAME=bearshare-github-runner
CONTAINER_NAME=bearshare-runner

docker build -t $IMAGE_NAME docker/runner

docker volume create bearshare-runner-work

docker run -dit \
  --name $CONTAINER_NAME \
  --restart unless-stopped \
  --memory=8g \
  --cpus=2 \
  -e RUNNER_NAME=bearshare-runner \
  -e RUNNER_REPO_URL=https://github.com/ickoxii/BearShare \
  -v /var/run/docker.sock:/var/run/docker.sock \
  --group-add $(getent group docker | cut -d: -f3) \
  $IMAGE_NAME
