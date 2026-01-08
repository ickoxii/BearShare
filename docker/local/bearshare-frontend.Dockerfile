# Create a build of the project
FROM node:20 AS build
WORKDIR /build
COPY frontend/package.json .
COPY frontend/yarn.lock .
RUN yarn install

COPY frontend/ .

EXPOSE 3000

ENTRYPOINT ["yarn", "run", "dev", "--", "--host"]
