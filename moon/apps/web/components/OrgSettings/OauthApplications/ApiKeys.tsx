import { OauthApplication } from '@gitmono/types/generated'
import { Link } from '@gitmono/ui/Link'
import { UIText } from '@gitmono/ui/Text'

import { DeveloperTokenButton } from '@/components/OrgSettings/OauthApplications/DeveloperTokenButton'
import * as SettingsSection from '@/components/SettingsSection'

export function ApiKeys({ oauthApplication }: { oauthApplication: OauthApplication }) {
  return (
    <SettingsSection.Section>
      <SettingsSection.Header className='p-3'>
        <SettingsSection.Title>
          API keys
          <UIText tertiary>
            Use API keys to make requests to the Campsite API.{' '}
            <Link href='https://developers.campsite.com' target='_blank' className='text-blue-500'>
              Docs &rsaquo;
            </Link>
            .
          </UIText>
        </SettingsSection.Title>
        <DeveloperTokenButton oauthApplication={oauthApplication} />
      </SettingsSection.Header>
    </SettingsSection.Section>
  )
}
