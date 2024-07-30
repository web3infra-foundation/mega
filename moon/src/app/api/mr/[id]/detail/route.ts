export const dynamic = 'force-dynamic' // defaults to auto
export const revalidate = 0

const endpoint = process.env.NEXT_PUBLIC_API_URL;

export async function GET(request: Request,  { params }: { params: { id: string } }) {
    const res = await fetch(`${endpoint}/api/v1/mr/${params.id}/detail`, {
    })
    const data = await res.json()

    return Response.json({ data })
}