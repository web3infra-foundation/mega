import { BaseStyles, ThemeProvider } from '@primer/react'
import { GetServerSideProps } from 'next'

import IssueDetailPage from '@/components/Issues/IssueDetailPage'
import { AppLayout } from '@/components/Layout/AppLayout'
import { AuthAppProviders } from '@/components/Providers/AuthAppProviders'
import { PageWithLayout } from '@/utils/types'

export const getServerSideProps: GetServerSideProps = async ({ query }) => {
  if (!query.link) {
    return {
      redirect: {
        destination: `/${query.org}/issue`,
        permanent: false
      }
    }
  }
  return {
    props: {
      link: query.link
    }
  }
}

const OrganizationIssueDetailPage: PageWithLayout<any> = ({ link }) => {
  return (
    <>
      <ThemeProvider>
        <BaseStyles>
          <IssueDetailPage link={link} key={link} />
        </BaseStyles>
      </ThemeProvider>
    </>
  )
}

OrganizationIssueDetailPage.getProviders = (page, pageProps) => {
  return (
    <AuthAppProviders {...pageProps}>
      <AppLayout {...pageProps}>{page}</AppLayout>
    </AuthAppProviders>
  )
}

export default OrganizationIssueDetailPage
