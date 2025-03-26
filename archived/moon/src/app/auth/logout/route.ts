import { redirect } from 'next/navigation'
export const revalidate = 0
export const dynamic = 'force-dynamic' // defaults to auto

export async function GET() {
    const endpoint = process.env.MEGA_HOST;
    redirect(`${endpoint}/auth/logout`)
}