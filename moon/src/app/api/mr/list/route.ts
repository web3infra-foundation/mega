export const dynamic = 'force-dynamic' // defaults to auto
export const revalidate = 0

import { type NextRequest } from 'next/server'

export async function GET(request: NextRequest) {
    const endpoint = process.env.MEGA_HOST;
    const searchParams = request.nextUrl.searchParams
    const status = searchParams.get('status')
    const res = await fetch(`${endpoint}/api/v1/mr/list?status=${status}`, {
    })
    const data = await res.json()

    return Response.json({ data })
}