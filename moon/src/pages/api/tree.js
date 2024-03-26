// /pages/api/tree.js

import axios from 'axios';

export default async function handler(req, res) {
    // 从 req.query 中提取 repo_path 和 object_id 参数
    const { repo_path, object_id } = req.query;

    try {
        // 构建请求 URL
        const apiUrl = `http://localhost:8000/api/v1/tree?repo_path=/projects/freighter&object_id=${encodeURIComponent(object_id)}`;

        // 发起对外部 API 的请求
        const response = await axios.get(apiUrl);

        // 提取响应数据
        const treeData = response.data;

        // 返回数据给客户端
        res.status(200).json(treeData);
    } catch (error) {
        // 如果发生错误，返回错误信息给客户端
        console.error('Error fetching tree data:', error);
        res.status(500).json({ error: 'Error fetching tree data' });
    }
}
