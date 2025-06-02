  # 构建阶段
  FROM node:20-alpine AS builder
  
  WORKDIR /app
  
  # 安装 pnpm（保持一致版本）
  RUN corepack enable && corepack prepare pnpm@9.7.1 --activate
  
  # 安装依赖
  COPY . .
  RUN pnpm install --frozen-lockfile

  # 构建：递归构建nextjs中的所有依赖
  RUN pnpm run build

  # 运行阶段
  FROM node:20-alpine AS runner
  WORKDIR /app
  ENV PORT=3000

  # 启用 pnpm
  RUN corepack enable && corepack prepare pnpm@9.7.1 --activate

  COPY --from=builder /app/apps/web/.next/standalone ./
  COPY --from=builder /app/apps/web/.next/static ./.next/static
  COPY --from=builder /app/apps/web/public ./public
  
  # 启动 SSR 服务
  WORKDIR /app/apps/web
  CMD ["node", "server.js"]
  