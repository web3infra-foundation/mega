'use client'

import { handleLogin } from '@/app/actions';

export default function AuthPage({ searchParams }) {
    const apiUrl = process.env.NEXT_PUBLIC_API_URL;
    const access_token = searchParams.access_token || "";
    const code = searchParams.code || "";
    const state = searchParams.state || "";

    if (code && state) {
        const targetUrl = `${apiUrl}/auth/github/callback?code=${code}&state=${state}`;
        window.location.href = targetUrl;
    } else if (access_token) {
        handleLogin(access_token)
    }
}
