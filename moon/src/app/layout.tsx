import '@/styles/globals.css'
import type { Metadata } from 'next'
import type React from 'react'
import { AntdRegistry } from '@ant-design/nextjs-registry';

export const metadata: Metadata = {
  title: {
    template: '%s - Mega',
    default: 'Mega',
  },
  description: '',
}

export default async function Layout({ children }: { children: React.ReactNode }) {

  return (
    <html
      lang="en"
      className="text-zinc-950 antialiased lg:bg-zinc-100 dark:bg-zinc-900 dark:text-white dark:lg:bg-zinc-950"
    >
      <head>
        <link rel="preconnect" href="https://rsms.me/" />
        <link rel="stylesheet" href="https://rsms.me/inter/inter.css" />
      </head>
      <body>
        <AntdRegistry>{children}</AntdRegistry>
      </body>
    </html>
  )
}
