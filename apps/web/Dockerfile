FROM node:20-slim AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:20-slim

WORKDIR /app
ENV HOST=0.0.0.0
ENV PORT=8080

COPY --from=builder /app/dist ./dist
COPY --from=builder /app/package*.json ./
RUN npm ci --omit=dev

CMD ["sh", "-c", "npx serve dist --single --listen tcp://0.0.0.0:${PORT}"]
