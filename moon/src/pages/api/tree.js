// /pages/api/tree.js

import axios from 'axios';

export default async function handler(req, res) {
    const { repo_path, object_id } = req.query;

    try {
        const apiUrl = `http://localhost:8000/api/v1/tree?path=${encodeURIComponent(repo_path)}`;

        const response = await axios.get(apiUrl);

        const treeData = response.data;

        res.status(200).json(treeData);
    } catch (error) {

        console.error('Error fetching tree data:', error);
        res.status(500).json({ error: 'Error fetching tree data' });
    }
}
