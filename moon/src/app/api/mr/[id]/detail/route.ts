export const dynamic = 'force-dynamic' // defaults to auto
export const revalidate = 0

export async function GET(request: Request,  { params }: { params: { id: string } }) {
    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/mr/${params.id}/detail`, {
    })
    const data = await res.json()

    return Response.json({ data })
}