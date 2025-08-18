FROM mcr.microsoft.com/playwright:v1.53.1-noble

WORKDIR /e2e

COPY package.json package-lock.json ./

RUN npm ci --no-audit --no-fund --prefer-offline

COPY . .

CMD ["npx", "playwright", "test"]
