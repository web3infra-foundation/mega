# ----------- Stage 1: Build the app -----------
  FROM node:20-alpine AS builder

  WORKDIR /app
  
  # 安装指定版本的 pnpm（9.7.1）
  RUN corepack enable && corepack prepare pnpm@9.7.1 --activate
  
  # 构建阶段需要传入 TIPTAP_TOKEN
  ARG TIPTAP_TOKEN
  
  # 拷贝源码
  COPY package.json pnpm-lock.yaml pnpm-workspace.yaml turbo.json ./

  COPY . .
  
  # 设置TipTap环境
  RUN pnpm config set "@tiptap-pro:registry" https://registry.tiptap.dev/
  RUN pnpm config set "//registry.tiptap.dev/:_authToken" "$TIPTAP_TOKEN"

  # 安装依赖
  RUN pnpm install --frozen-lockfile

  RUN pnpm run lint
  
  # 构建 Next.js 应用（SSR 模式）
  # 进入 apps/web 构建 Next.js
  WORKDIR /app
  RUN pnpm run build

  
  # ----------- Stage 2: Run the app with only needed files -----------
  FROM node:20-alpine AS runner
  
  WORKDIR /app
  
  ENV PORT=3000
  
  # 安装 pnpm（保持一致版本）
  RUN corepack enable && corepack prepare pnpm@9.7.1 --activate
  
  # 拷贝构建产物和依赖
  COPY --from=builder /app /app
  # COPY --from=builder /app/apps/web/public ./public
  # COPY --from=builder /app/apps/web/.next ./.next
  # COPY --from=builder /app/apps/web/node_modules ./node_modules
  # COPY --from=builder /app/apps/web/package.json ./package.json
  # COPY --from=builder /app/apps/web/next.config.js ./next.config.js
  
  # 启动 SSR 服务
  WORKDIR /app/apps/web
  CMD ["pnpm", "start"]
  