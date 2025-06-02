# syntax = docker/dockerfile:1

FROM node:20-slim AS base

# configure pnpm
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable

ARG TIPTAP_PRIVATE_REGISTRY_KEY


# Throw-away build stage to reduce size of final image
FROM base AS builder
# Install packages needed to build node modules
RUN apt-get update -qq && \
  apt-get install -y build-essential pkg-config python-is-python3 ca-certificates
WORKDIR /app
RUN pnpm install turbo@2.0.9 --global
COPY . .
RUN turbo prune @gitmono/web --docker
WORKDIR /app


FROM base AS installer
RUN apt-get update -qq && \
  apt-get install -y build-essential pkg-config python-is-python3 ca-certificates
WORKDIR /app

COPY --from=builder /app/out/json/ .
COPY --from=builder /app/out/pnpm-lock.yaml ./pnpm-lock.yaml
COPY --from=builder /app/package.json ./
COPY --from=builder /app/patches ./patches
COPY --from=builder /app/pnpm-workspace.yaml ./


RUN pnpm config set "@tiptap-pro:registry" https://registry.tiptap.dev/
RUN pnpm config set "//registry.tiptap.dev/:_authToken" CjqFil0n7z4GGur+RPric21fBBeSB4R4FoNdRYOE1bQEz6AXLCoANCc+o9rLIg6Q
RUN pnpm install
RUN npx patch-package

COPY --from=builder /app/out/full/ .

RUN npx turbo run build --filter=@gitmono/web


RUN apt-get update -y && apt-get install -y ca-certificates



FROM base AS runner

# Set production environment
ENV NODE_ENV="production"

WORKDIR /app

# Copy built application
COPY --from=installer --chown=nextjs:nodejs /app/apps/web/.next/standalone ./
COPY --from=installer --chown=nextjs:nodejs /app/apps/web/.next/static ./apps/web/.next/static
COPY --from=installer --chown=nextjs:nodejs /app/apps/web/public ./apps/web/public

# Start the server by default, this can be overwritten at runtime
EXPOSE 3000
ENV PORT=3000

ENTRYPOINT [ "node", "/app/apps/web/server.js" ]
