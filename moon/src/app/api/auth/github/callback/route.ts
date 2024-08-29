
// import { cookies } from "next/headers";
import { type NextRequest } from 'next/server'

export async function POST(request: NextRequest) {
    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const searchParams = request.nextUrl.searchParams;
    const code = searchParams.get("code");
    const state = searchParams.get("state");
    const res = await fetch(`${endpoint}/auth/github/callback?code=${code}&state=${state}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
    })
    const data = await res.json()

    return Response.json({ data })
}