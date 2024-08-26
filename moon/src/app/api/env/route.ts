export const revalidate = 0
export const dynamic = 'force-dynamic' // defaults to auto

export async function GET() {
    return new Response(JSON.stringify({
        mega_host: process.env.MEGA_HOST,
        callback_url: process.env.CALLBACK_URL,
    }), {
        headers: { 'Content-Type': 'application/json' }
    });
}