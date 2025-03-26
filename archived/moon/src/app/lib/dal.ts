import 'server-only'

import { cookies } from 'next/headers'
import { cache } from 'react'
import { redirect } from 'next/navigation'

export const verifySession = cache(async () => {
    const cookieStore = await cookies()
    const session = cookieStore.get('SESSION')?.value
    if (!session) {
        redirect('/login')
    }
    return { session }
})

export const isLoginIn = async () => {
    const cookieStore = await cookies()
    const session = cookieStore.get('SESSION')?.value
    return session != null 
}
