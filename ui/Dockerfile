FROM oven/bun:1.3.13 AS builder
WORKDIR /app

COPY package.json bun.lock ./
RUN bun install --frozen-lockfile

COPY . .
RUN bun run build

FROM oven/bun:1.3.13-slim
WORKDIR /app

COPY --from=builder /app/.output ./.output

ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

CMD ["bun", "run", ".output/server/index.mjs"]
