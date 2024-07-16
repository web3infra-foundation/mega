export const dynamic = 'force-dynamic' // defaults to auto

const endpoint = process.env.NEXT_MEGA_API_URL;

export async function GET(request: Request,  { params }: { params: { id: string } }) {
    const res = await fetch(`${endpoint}/api/v1/mr/${params.id}/files`, {
    })
    const data = await res.json()

    return Response.json({ data })
}