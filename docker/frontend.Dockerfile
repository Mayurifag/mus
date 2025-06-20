FROM node:24-alpine

WORKDIR /app

EXPOSE 5173

CMD ["sh", "-c", "npm i --no-fund && npm run dev -- --host 0.0.0.0 --port 5173"]
