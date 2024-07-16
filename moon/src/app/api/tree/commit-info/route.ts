import { type NextRequest } from 'next/server'

export const dynamic = 'force-dynamic' // defaults to auto
export const revalidate = 0

const endpoint = process.env.NEXT_MEGA_API_URL;

export async function GET(request: NextRequest) {
    const searchParams = request.nextUrl.searchParams
    const path = searchParams.get('path')

    const res = await fetch(`${endpoint}/api/v1/tree/commit-info?path=${path}`, {
    })
    const data = await res.json()

    return Response.json({ data })
}