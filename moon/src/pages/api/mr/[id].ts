import axios from 'axios';

export default async function handler(req, res) {
    const endpoint = process.env.NEXT_MEGA_API_URL;
    const { id } = req.query;

    try {
        const apiUrl = `${endpoint}/api/v1/mr-detail?id=${id}`;
        const response = await axios.get(apiUrl);
        const data = response.data;
        res.status(200).json(data);
    } catch (error) {
        console.error('Error fetching data:', error);
        res.status(500).json({ error: 'Error fetching data' });
    }
}
