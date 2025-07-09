import { Theme } from '@radix-ui/themes'
import Head from 'next/head'

import CodeView from '@/components/CodeView'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

const OrganizationTestPage: PageWithLayout<any> = () => {
  return (
    <>
      <Head>
        <title>Code</title>
      </Head>

      <Theme>
        <CodeView />
      </Theme>
    </>
  )
}

OrganizationTestPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
        <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationTestPage
