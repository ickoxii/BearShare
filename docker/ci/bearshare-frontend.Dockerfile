# Stage 1
FROM node:20 AS build

WORKDIR /build
COPY frontend/package.json frontend/yarn.lock .
RUN yarn install --frozen-lockfile

COPY frontend .
RUN yarn run build

# Stage 2
FROM nginx:alpine

COPY --from=build /build/dist /usr/share/nginx/html

EXPOSE 80
