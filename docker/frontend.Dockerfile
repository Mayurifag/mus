# syntax=docker/dockerfile:1

FROM node:26-alpine3.24

WORKDIR /app

RUN npm install -g npm@12.2.2

COPY frontend/package*.json ./
RUN --mount=type=cache,target=/root/.npm npm ci --no-fund

EXPOSE 5173

CMD ["sh", "-c", "test -x node_modules/.bin/vite || npm install --no-fund; npm run dev -- --host 0.0.0.0 --port 5173"]
