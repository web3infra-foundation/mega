import { Inter } from 'next/font/google'
import { GoogleAnalytics } from '@next/third-parties/google'

import clsx from 'clsx'

import '@/styles/tailwind.css'

const inter = Inter({
  subsets: ['latin'],
  display: 'swap',
  variable: '--font-inter',
})

export const metadata = {
  title: {
    template: '%s - Mega',
    default: 'Mega - an unofficial open source implementation of Google Piper',
  },
  description:
    'Mega is an unofficial open source implementation of Google Piper. It is a monorepo & monolithic codebase management system that supports Git. Mega is designed to manage large-scale codebases, streamline development, and foster collaboration.',
}

export default function RootLayout({ children }) {
  return (
    <html
      lang="en"
      className={clsx('h-full bg-gray-50 antialiased', inter.variable)}
    >
      <body className="flex h-full flex-col">
        <div className="flex min-h-full flex-col">{children}</div>
      </body>
      <GoogleAnalytics gaId="G-WCSCZGFL72" />
    </html>
  )
}
