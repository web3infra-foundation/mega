'use client'
import '@/styles/globals.css'
import type React from 'react'

import { ApplicationLayout } from './application-layout'
import { AntdRegistry } from '@ant-design/nextjs-registry';


export default function Layout({ children }: { children: React.ReactNode }) {

  return (
    <html
      lang="en"
      className="text-zinc-950 antialiased lg:bg-zinc-100 dark:bg-zinc-900 dark:text-white dark:lg:bg-zinc-950"
    >
      <head>
      </head>
      <body>
        <AntdRegistry>
          <ApplicationLayout >
            {children}
          </ApplicationLayout>
        </AntdRegistry>
      </body>
    </html>
  )
}
