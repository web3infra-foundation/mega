import 'server-only'

import { cookies } from 'next/headers'
import { cache } from 'react'
import { redirect } from 'next/navigation'

export const verifySession = cache(async () => {
    const session = cookies().get('SESSION')?.value
    if (!session) {
        redirect('/')
    }
    return { session }
})


export const isLoginIn = () => {
    const session = cookies().get('SESSION')?.value
    return session != null 
}
