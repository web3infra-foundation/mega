export const dynamic = 'force-dynamic' // defaults to auto
export const revalidate = 0

import { type NextRequest } from 'next/server'

export async function POST(request: NextRequest) {
    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const body = await request.json();
    const res = await fetch(`${endpoint}/api/v1/mr/list`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(body),
    })
    const data = await res.json()

    return Response.json({ data })
}