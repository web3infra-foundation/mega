'use client'

import { handleLogin } from '@/app/actions';
import { useCallback, useEffect } from 'react';
import { message } from "antd";

export default function AuthPage({ searchParams }) {
    const [messageApi, contextHolder] = message.useMessage();


    const error = useCallback((err_msg) => {
        messageApi.open({
            type: 'error',
            content: err_msg,
        });
    }, [messageApi]);
    const code = searchParams.code;
    const state = searchParams.state;

    useEffect(() => {
        async function fetchData() {
            try {
                const res = await fetch(`/api/auth/github/callback?code=${code}&state=${state}`, {
                    method: 'POST',
                });
                if (!res.ok) {
                    throw new Error('Failed to fetch data');
                }
                const result = await res.json();
                let data = result.data;
                if (data.req_result) {
                    handleLogin(data.data)
                } else {
                    error(data.err_message)
                }
            } catch (error) {
                console.error('Error fetching data:', error);
            }
        }
        if (code && state) {
            fetchData();
        }
    }, [code, state, error]);
    return <>
        {contextHolder}
    </>
}
