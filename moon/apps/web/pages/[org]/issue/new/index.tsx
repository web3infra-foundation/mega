import { BaseStyles, ThemeProvider } from '@primer/react'
import { useRouter } from 'next/router'

import '@primer/primitives/dist/css/functional/themes/light.css'

import IssueNewPage from '@/components/Issues/IssueNewPage'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

const OrganizationIssueNewPage: PageWithLayout<any> = () => {
  const router = useRouter()

  return (
    <>
      <IssueNewPage key={router.pathname} />
    </>
  )
}

OrganizationIssueNewPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <ThemeProvider>
        <BaseStyles>
          <AppLayout {...pageProps}>{page}</AppLayout>
        </BaseStyles>
      </ThemeProvider>
    </AuthAppProviders>
  )
}

export default OrganizationIssueNewPage
