import { verifySession } from '@/app/lib/dal'
import { NextResponse } from 'next/server'

const endpoint = process.env.MEGA_INTERNAL_HOST;

export async function POST(request: Request, { params }: { params: { id: string } }) {
    const session = await verifySession()
    if (!session) return Response.json({})

    const cookieHeader = request.headers.get('cookie') || '';

    const res = await fetch(`${endpoint}/api/v1/user/ssh/${params.id}/delete`, {
        headers: {
            'Cookie': cookieHeader,
        },
        method: 'POST'
    })
    if (!res.ok) {
        return new NextResponse(
            JSON.stringify({ error: res.statusText }),
            { status: res.status }
        );
    }
    const data = await res.json()
    return Response.json({ data })
}