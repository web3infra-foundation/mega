import { type NextRequest } from 'next/server'
export const revalidate = 0
export const dynamic = 'force-dynamic' // defaults to auto

export async function GET(request: NextRequest) {
    const endpoint = process.env.MEGA_INTERNAL_HOST;

    const searchParams = request.nextUrl.searchParams
    const path = searchParams.get('path')

    const res = await fetch(`${endpoint}/api/v1/tree?path=${path}`, {
    })
    const data = await res.json()

    return Response.json({ data })
}