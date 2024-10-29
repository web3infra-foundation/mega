export async function POST(request: Request, { params }: { params: { id: string } }) {
    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/issue/${params.id}/close`, {
        method: 'POST',
    })
    const data = await res.json()

    return Response.json({ data })
}