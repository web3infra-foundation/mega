# Dockerfile

FROM node:20-alpine

# 定义构建参数，传入私钥
ARG TIPTAP_TOKEN

# 设置工作目录
WORKDIR /app

# 生成 .npmrc，注入私钥（只在构建时使用）
RUN echo "@tiptap-pro:registry=https://registry.tiptap.dev/" > .npmrc && \
    echo "//registry.tiptap.dev/:_authToken=${TIPTAP_TOKEN}" >> .npmrc

# 复制 package.json 和 pnpm-lock.yaml
COPY package.json pnpm-lock.yaml ./

# 安装 pnpm
RUN npm install -g pnpm@9.7.1

# 安装依赖
RUN pnpm install --frozen-lockfile

# 复制剩余代码
COPY . .

# 构建项目（Next.js）
RUN pnpm run build

# 删除 .npmrc 以防泄露
RUN rm -f .npmrc

# 生产环境使用的端口
EXPOSE 3000

# 运行服务（生产环境）
CMD ["pnpm", "start"]
