import { Inter } from 'next/font/google'
import clsx from 'clsx'
import { Analytics } from '@vercel/analytics/react';
import Script from "next/script";

import '@/styles/tailwind.css'

const inter = Inter({
  subsets: ['latin'],
  display: 'swap',
  variable: '--font-inter',
})

export const metadata = {
  title: {
    template: '%s - Mega',
    default: 'Mega - Monorepo Engine for Enterprise and Individual.',
  },
  description:
    'Mega is an engine for managing a monorepo. It functions similarly to Google s Piper and helps to streamline Git and trunk-based development for large-scale projects.',
}

const GA_MEASUREMENT_ID = process.env.NEXT_PUBLIC_MEASUREMENT_ID;


export default function RootLayout({ children }) {
  return (
    <html
      lang="en"
      className={clsx('h-full bg-gray-50 antialiased', inter.variable)}
    >
      <body className="flex h-full flex-col">
        <div className="flex min-h-full flex-col">{children}</div>
      </body>
      <Script
        src={`https://www.googletagmanager.com/gtag/js?id=${GA_MEASUREMENT_ID}`}
        strategy="afterInteractive"
      />
      <Script id="google-analytics" strategy="afterInteractive">
        {`
            window.dataLayer = window.dataLayer || [];
            function gtag(){window.dataLayer.push(arguments);}
            gtag('js', new Date());
            gtag('config', '${GA_MEASUREMENT_ID}');
          `}
      </Script>

      <Analytics />
    </html>
  )
}
