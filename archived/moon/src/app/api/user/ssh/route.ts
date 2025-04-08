import { verifySession } from '@/app/lib/dal'
import { NextResponse, type NextRequest } from 'next/server'
export const revalidate = 0
export const dynamic = 'force-dynamic' // defaults to auto

const endpoint = process.env.MEGA_INTERNAL_HOST;

export async function POST(request: NextRequest) {
    const session = await verifySession()
    if (!session) return Response.json({})

    const cookieHeader = request.headers.get('cookie') || '';
    const body = await request.json();

    const res = await fetch(`${endpoint}/api/v1/user/ssh`, {
        headers: {
            'Cookie': cookieHeader,
            'Content-Type': 'application/json',
        },
        method: 'POST',
        body: JSON.stringify(body),
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


export async function GET(request: NextRequest) { 
    const session = await verifySession()
    if (!session) return Response.json({})

    const cookieHeader = request.headers.get('cookie') || '';

    const res = await fetch(`${endpoint}/api/v1/user/ssh`, {
        headers: {
            'Cookie': cookieHeader,
        },
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