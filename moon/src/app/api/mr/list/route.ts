export const dynamic = 'force-dynamic' // defaults to auto
export const revalidate = 0

import { type NextRequest } from 'next/server'

const endpoint = process.env.NEXT_MEGA_API_URL;

export async function GET(request: NextRequest) {
    const searchParams = request.nextUrl.searchParams
    const status = searchParams.get('status')
    const res = await fetch(`${endpoint}/api/v1/mr/list?status=${status}`, {
    })
    const data = await res.json()

    return Response.json({ data })
}