export const revalidate = 0
export const dynamic = 'force-dynamic' // defaults to auto

import { NextRequest, NextResponse } from 'next/server';

export async function GET(request: NextRequest) {
    const endpoint = process.env.MEGA_HOST;

    const currentUrl = new URL(request.url);

    const redirectUrl = new URL(`${endpoint}/auth/authorized`, request.url);

    currentUrl.searchParams.forEach((value, key) => {
        redirectUrl.searchParams.set(key, value);
    });

    return NextResponse.redirect(redirectUrl.toString());
}