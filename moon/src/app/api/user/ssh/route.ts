import { type NextRequest } from 'next/server'
export const revalidate = 0
export const dynamic = 'force-dynamic' // defaults to auto

export async function POST(request: NextRequest) {
    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const cookieHeader = request.headers.get('cookie') || '';
    const body = await request.json();

    const res = await fetch(`${endpoint}/api/v1/user/ssh`, {
        headers: {
            'Cookie': cookieHeader,
            'Content-Type': 'application/json',
        },
        method: 'POST',
        body: body,
    })
    const data = await res.json()
    return Response.json({ data })
}