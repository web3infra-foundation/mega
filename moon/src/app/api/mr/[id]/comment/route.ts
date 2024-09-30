import { verifySession } from "@/app/lib/dal";

export async function POST(request: Request,  { params }: { params: { id: string } }) {
    const session = await verifySession()
    const jsonData = await request.json();

    // Check if the user is authenticated
    if (!session) {
      // User is not authenticated
      return new Response(null, { status: 401 })
    }

    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/mr/${params.id}/comment`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(jsonData),
    })
    const data = await res.json()

    return Response.json({ data })
}