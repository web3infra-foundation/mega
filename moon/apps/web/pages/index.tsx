import { SCOPE_COOKIE_NAME } from '@gitmono/config';
import { ApiErrorTypes } from '@gitmono/types';
import { GetServerSideProps } from 'next';
import { userAgentFromString } from 'next/server';
import { apiCookieHeaders } from '@/utils/apiCookieHeaders';
import { apiClient, signinUrl } from '@/utils/queryClient';

export default function IndexPage() {
  return <></>
}

export const getServerSideProps: GetServerSideProps = async ({ req, query }) => {
  try {
    const { device } = userAgentFromString(req.headers['user-agent'])
    const headers = apiCookieHeaders(req.cookies)

    const organizations = await apiClient.organizationMemberships
      .getOrganizationMemberships()
      .request({ headers })
      .then((res) =>
        res.map(m => m.organization)
          .filter(o => o !== null)
      )

    // if we have orgs redirect to one of the user orgs,
    // otherwise redirect to the new org page
    if (organizations.length > 0) {
      let org = organizations[organizations.length - 1]
      const scope = req.cookies[SCOPE_COOKIE_NAME]

      if (scope) {
        const scopeOrg = organizations.find((o) => o?.slug == scope)

        if (scopeOrg) {
          org = scopeOrg
        }
      }

      if (device.type === 'mobile') {
        return {
          redirect: {
            destination: `/${org?.slug}/${query.path ?? 'home'}`,
            permanent: false
          }
        }
      }
      return {
        redirect: {
          destination: `/${org?.slug}/${query.path ?? ''}`,
          permanent: false
        }
      }
    } else {
      const userData = await apiClient.users
        .getMe()
        .request({ headers })
        .then((res) => res)

      await apiClient.organizations.postInvitations().request(
        'mega',
        {
          invitations: [
            {
              email: userData.email,
              role: 'member'
            }
          ]
        },
        { headers }
      )

      const invitations = await apiClient.users.getMeOrganizationInvitations().request({ headers })

      for (let i of invitations) {
        if (i.organization?.slug === 'mega') {
          await apiClient.invitationsByToken.postInvitationsByTokenAccept().request(i.token!!, { headers })
        }
      }

      if (device.type === 'mobile') {
        return {
          redirect: {
            destination: `/mega/${query.path ?? 'home'}`,
            permanent: false
          }
        }
      }
      return {
        redirect: {
          destination: `/mega/${query.path ?? ''}`,
          permanent: false
        }
      }
    }
  } catch (e: any) {
    if (e.name === ApiErrorTypes.AuthenticationError) {
      return {
        redirect: {
          permanent: false,
          destination: signinUrl({ from: req?.url })
        }
      }
    } else if (e.name === ApiErrorTypes.NotFoundError) {
      return { notFound: true }
    } else {
      throw e
    }
  }
}
