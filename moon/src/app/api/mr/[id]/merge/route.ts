import { verifySession } from "@/app/lib/dal";

export const dynamic = 'force-dynamic' // defaults to auto

type Params = Promise<{ id: string }>

export async function POST(request: Request, props: { params: Params }) {
    const params = await props.params;
    
    const session = await verifySession()

    // Check if the user is authenticated
    if (!session) {
        // User is not authenticated
        return new Response(null, { status: 401 })
    }

    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/mr/${params.id}/merge`, {
        method: 'POST',
        headers: {
            'Cookie': request.headers.get('cookie') || '',
        },
    })
    const data = await res.json()

    return Response.json({ data })
}