export const dynamic = 'force-dynamic' // defaults to auto


export async function POST(request: Request,  { params }: { params: { id: string } }) {
    const endpoint = process.env.MEGA_HOST;
    const res = await fetch(`${endpoint}/api/v1/mr/${params.id}/merge`, {
        method: 'POST',
    })
    const data = await res.json()

    return Response.json({ data })
}