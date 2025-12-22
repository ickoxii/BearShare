FROM node:20 AS build

WORKDIR /build
COPY frontend/package*.json ./
RUN npm install

COPY frontend .
RUN npm run build

FROM node:20

WORKDIR /app

# Copy only what is needed to run preview
COPY --from=build /build/package*.json ./
COPY --from=build /build/node_modules ./node_modules
COPY --from=build /build/dist ./dist

EXPOSE 3000

CMD ["npm", "run", "preview", "--", "--host", "0.0.0.0", "--port", "3000"]
