FROM node:24-alpine

WORKDIR /app

RUN npm install -g npm@latest markdownlint-cli2

COPY frontend/package*.json ./
RUN npm ci --no-fund

EXPOSE 5173

CMD ["sh", "-c", "test -x node_modules/.bin/vite || npm install --no-fund; npm run dev -- --host 0.0.0.0 --port 5173"]
