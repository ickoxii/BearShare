# Create a build of the project
FROM node:20 AS build
WORKDIR /build
COPY frontend .

RUN npm install

EXPOSE 3000

ENTRYPOINT ["npm", "run", "dev", "--", "--host"]
