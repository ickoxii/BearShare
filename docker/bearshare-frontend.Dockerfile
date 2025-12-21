FROM python:3.11.14-alpine3.23

WORKDIR /build
COPY . .

WORKDIR /build/frontend

ENTRYPOINT python -m http.server 3000
