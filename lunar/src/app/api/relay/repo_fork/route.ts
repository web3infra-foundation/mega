export const revalidate = 0

import { NextRequest } from "next/server";

const endpoint = process.env.MEGA_API_URL;

export async function GET(request: NextRequest) {
    const searchParams = request.nextUrl.searchParams
    const identifier = searchParams.get('identifier')
    const port = searchParams.get('port')
    const res = await fetch(`${endpoint}/api/v1/ztm/repo_folk?identifier=${identifier}&port=${port}`, {
    })
    const data = await res.json()
    
    return Response.json({ data })
}