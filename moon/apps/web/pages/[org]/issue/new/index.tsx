import { BaseStyles, ThemeProvider } from '@primer/react'
import { useRouter } from 'next/router'

import IssueNewPage from '@/components/Issues/IssueNewPage'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

const OrganizationIssueNewPage: PageWithLayout<any> = () => {
  const router = useRouter()

  return (
    <>
      <ThemeProvider>
          <BaseStyles>
            <IssueNewPage key={router.pathname} />
      </BaseStyles>
    </ThemeProvider>
    </>
  )
}

OrganizationIssueNewPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationIssueNewPage
