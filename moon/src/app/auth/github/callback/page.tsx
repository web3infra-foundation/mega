'use client'

import { handleLogin } from '@/app/actions';
import { useEffect, useState } from 'react';

interface enviroment {
    mega_host: string;
    callback_url: string;
}

export default function AuthPage({ searchParams }) {
    const [data, setData] = useState<enviroment | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        async function fetchData() {
            try {
                const res = await fetch(`/api/env`);
                if (!res.ok) {
                    throw new Error('Failed to fetch data');
                }
                const result = await res.json();
                setData(result);
            } catch (error) {
                console.error('Error fetching data:', error);
            } finally {
                setLoading(false);
            }
        }

        fetchData();
    }, []);

    if (loading) return <div>Loading...</div>;
    if (!data) return <div>Env Route Error</div>;

    const access_token = searchParams.access_token || "";
    const code = searchParams.code || "";
    const state = searchParams.state || "";

    if (code && state) {
        const targetUrl = `${data.mega_host}/auth/github/callback?code=${code}&state=${state}`;
        window.location.href = targetUrl;
    } else if (access_token) {
        handleLogin(access_token)
    }
}
