export const revalidate = 0

const endpoint = process.env.RELAY_API_URL;

export async function GET(request: Request, ) {
    const res = await fetch(`${endpoint}/relay/repo_list`, {
    })
    const data = await res.json()
    return Response.json({ data })
}