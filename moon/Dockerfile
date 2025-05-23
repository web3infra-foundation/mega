  FROM node:20-alpine AS runner
  
  WORKDIR /app
  
  ENV PORT=3000
  
  # 安装 pnpm（保持一致版本）
  RUN corepack enable && corepack prepare pnpm@9.7.1 --activate
  
  # 拷贝构建产物和依赖
  COPY . .
  
  # 启动 SSR 服务
  WORKDIR /app/apps/web
  CMD ["pnpm", "start"]
  