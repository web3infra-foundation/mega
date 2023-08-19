/** @type {import('next').NextConfig} */
const nextConfig = {}
const withCss = require('@zeit/next-css')
const { createProxyMiddleware } = require('http-proxy-middleware');
const axios = require('axios');

if (typeof require !== 'undefined') {
    require.extensions['.css'] = file => { }
}

module.exports = withCss({})

module.exports = nextConfig

module.exports = {
    async rewrites() {
        // 代理配置
        return [
            {
                source: '/api/:path*',
                destination: 'http://api.gitmega.dev/api/:path*', // 修改为实际的后端地址
            },
        ];
    },
};