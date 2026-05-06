FROM --platform=linux/amd64 node:20-alpine
WORKDIR /app

COPY .output ./.output

ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

CMD ["node", ".output/server/index.mjs"]
