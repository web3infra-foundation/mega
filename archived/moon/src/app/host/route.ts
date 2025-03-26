export const revalidate = 0

export async function GET() {
    const endpoint = process.env.MEGA_HOST;
    return Response.json({ endpoint })
}