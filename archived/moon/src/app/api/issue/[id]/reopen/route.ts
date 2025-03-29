type Params = Promise<{ id: string }>

export async function POST(request: Request, props: { params: Params }) {
    const params = await props.params

    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/issue/${params.id}/reopen`, {
        method: 'POST',
        headers: {
            'Cookie': request.headers.get('cookie') || '',
        },
    })
    const data = await res.json()

    return Response.json({ data })
}