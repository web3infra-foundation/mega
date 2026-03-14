import { useRouter } from 'next/navigation'

import { Button } from '@gitmono/ui'

import * as SettingsSection from '@/components/SettingsSection'
import { useClaStatus } from '@/hooks/Cla/useClaStatus'

const ClaStatusSection = () => {
  const { data: claStatusData, isLoading } = useClaStatus()
  const signed = claStatusData?.data?.cla_signed ?? false
  const signedAt = claStatusData?.data?.cla_signed_at
  const router = useRouter()

  return (
    <SettingsSection.Section>
      <SettingsSection.Header>
        <SettingsSection.Title>Contributor License Agreement (CLA)</SettingsSection.Title>
      </SettingsSection.Header>

      <SettingsSection.Description>
        Your CLA signing status. A signed CLA is required to contribute to projects.
      </SettingsSection.Description>

      <SettingsSection.Separator />

      <SettingsSection.Body>
        <div className='flex items-center justify-between'>
          <div className='flex items-center gap-3'>
            {isLoading ? (
              <span className='text-secondary text-sm'>Loading status...</span>
            ) : signed ? (
              <>
                <span className='inline-flex items-center gap-1.5 rounded-full bg-green-100 px-3 py-1 text-sm font-medium text-green-800 dark:bg-green-900/30 dark:text-green-400'>
                  <span className='h-2 w-2 rounded-full bg-green-500' />
                  Signed
                </span>
                {signedAt && (
                  <span className='text-secondary text-xs'>
                    {new Date(signedAt * 1000).toLocaleDateString(undefined, {
                      year: 'numeric',
                      month: 'long',
                      day: 'numeric'
                    })}
                  </span>
                )}
              </>
            ) : (
              <span className='inline-flex items-center gap-1.5 rounded-full bg-amber-100 px-3 py-1 text-sm font-medium text-amber-800 dark:bg-amber-900/30 dark:text-amber-400'>
                <span className='h-2 w-2 rounded-full bg-amber-500' />
                Not Signed
              </span>
            )}
          </div>

          {!signed && !isLoading && <Button onClick={() => router.push('/me/settings/cla/sign')}>Sign CLA</Button>}
        </div>
      </SettingsSection.Body>
    </SettingsSection.Section>
  )
}

export default ClaStatusSection
