import { verifySession } from "@/app/lib/dal";
import { redirect } from "next/navigation";
import { NextResponse } from "next/server";

export async function POST(request: Request, { params }: { params: { id: string } }) {
    const session = await verifySession()
    const jsonData = await request.json();

    const endpoint = process.env.MEGA_INTERNAL_HOST;
    const res = await fetch(`${endpoint}/api/v1/issue/${params.id}/comment`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            'Cookie': request.headers.get('cookie') || '',
        },
        body: JSON.stringify(jsonData),
    })
    const data = await res.json()

    return Response.json({ data })
}