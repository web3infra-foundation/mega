import Head from 'next/head'

import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'
import MrView from '@/components/MrView'


const OrganizationMrPage: PageWithLayout<any> = () => {

  return (
    <>
      <Head>
        <title>Mr</title>
      </Head>

      <MrView/>
    </>
  )
}

OrganizationMrPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationMrPage
