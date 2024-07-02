import axios from 'axios';

export default async function handler(req, res) {
    const endpoint = process.env.NEXT_MEGA_API_URL;
    const { path } = req.query;
    try {
        const apiUrl = `${endpoint}/api/v1/blob?path=${path}`;
        const response = await axios.get(apiUrl);
        const data = response.data;
        res.status(200).json(data);
    } catch (error) {

        console.error('Error fetching blob data:', error);
        res.status(500).json({ error: 'Error fetching blob data' });
    }
}
