type Params = Promise<{ id: string }>

export async function POST(request: Request, props: { params: Params }) {
    const params = await props.params

    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/issue/${params.id}/close`, {
        method: 'POST',
    })
    const data = await res.json()

    return Response.json({ data })
}