import { verifySession } from "@/app/lib/dal";

export const dynamic = 'force-dynamic' // defaults to auto

export async function POST(request: Request) {
    const session = await verifySession()

    // Check if the user is authenticated
    if (!session) {
        // User is not authenticated
        return new Response(null, { status: 401 })
    }
    const jsonData = await request.json();

    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/issue/new`, {
        method: 'POST',
        headers: {
            'Cookie': request.headers.get('cookie') || '',
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(jsonData)
    })
    const data = await res.json()

    return Response.json({ data })
}