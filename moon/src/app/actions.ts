'use server'

import { cookies } from 'next/headers'
import { redirect } from 'next/navigation';

export async function handleLogin(sessionData) {
  const encryptedSessionData = encrypt(sessionData) // Encrypt your session data
  const cookieStore = await cookies()
  cookieStore.set('access_token', encryptedSessionData, {
    httpOnly: true,
    secure: process.env.NODE_ENV === 'production',
    maxAge: 60 * 60 * 24 * 7, // One week
    path: '/',
  })
  redirect('/')
  // Redirect or handle the response after setting the cookie
}

export async function get_access_token() {
  const cookieStore = await cookies()
  const token = cookieStore.get('access_token')
  return token
}


// TODO encrypt access_token
function encrypt(sessionData) {
  return sessionData
}