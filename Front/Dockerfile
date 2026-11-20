# syntax=docker/dockerfile:1
# Front image (ADR-0012): build with Node, run only the bundled SSR server as a non-root user.

FROM node:26-alpine AS build
WORKDIR /src
COPY package.json package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY . .
RUN npm run build

FROM node:26-alpine
WORKDIR /app
ENV NODE_ENV=production \
    PORT=4000
# The server bundle is self-contained: node_modules is not needed at runtime.
COPY --from=build --chown=node:node /src/dist/Front ./dist/Front
USER node
EXPOSE 4000
CMD ["node", "dist/Front/server/server.mjs"]
