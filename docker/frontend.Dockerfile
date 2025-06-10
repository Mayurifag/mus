FROM node:20-alpine

WORKDIR /app

COPY frontend/package.json frontend/package-lock.json* ./

RUN npm i --no-fund

COPY frontend/ ./

EXPOSE 5173

CMD ["npm", "run", "dev", "--", "--host"]
