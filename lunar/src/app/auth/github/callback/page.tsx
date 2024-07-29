'use client'

import { useSearchParams } from 'next/navigation';
import { useRouter } from 'next/navigation';

export default function AuthPage() {
    const router = useRouter();
    const apiUrl = process.env.NEXT_PUBLIC_API_URL;
    const searchParams = useSearchParams();
    const access_token = searchParams.get('access_token') || "";
    const code = searchParams.get('code') || "";
    const state = searchParams.get('state') || "";

    if (code && state) {
        const targetUrl = `${apiUrl}/auth/github/callback?code=${code}&state=${state}`;
        window.location.href = targetUrl;
    } else if (access_token) {
        localStorage.setItem('access_token', access_token);
        router.push('/');
    }
}
