import '@/styles/globals.css'
import type { Metadata } from 'next'
import { AntdRegistry } from '@ant-design/nextjs-registry';
import { TreeStoreProvider } from '@/app/providers/tree-store-providers';

import { GoogleAnalytics } from "@next/third-parties/google";

const google_analytics_id = process.env.NEXT_PUBLIC_GOOGLE_ANALYTICS_ID || '';

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
        <TreeStoreProvider>
          <AntdRegistry>
            {children}
          </AntdRegistry>
        </TreeStoreProvider>
      </body>

      <GoogleAnalytics gaId={google_analytics_id} />
    </html>
  )
}
