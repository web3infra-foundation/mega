// import { type NextRequest } from 'next/server'

import { cookies } from "next/headers";

const endpoint = process.env.NEXT_PUBLIC_API_URL;

export async function GET(request: Request) {
    const cookieStore = cookies();
    const access_token = cookieStore.get('access_token');

    const res = await fetch(`${endpoint}/auth/github/user`, {
        next: { revalidate: 300 },
        headers: {
            'Authorization': `Bearer ${access_token?.value}`,
            'Content-Type': 'application/json',
        },
    })

    const data = await res.json()

    return Response.json({ data })
}