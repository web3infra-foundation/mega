import { RAILS_AUTH_URL } from '@gitmono/config'
import { useIsDesktopApp } from '@gitmono/ui/src/hooks'

interface IntegrationAuthParams {
  auth_url: string
  success_path?: string
  enable_notifications?: boolean
}

export function useIntegrationAuthUrl({ enable_notifications, ...rest }: IntegrationAuthParams) {
  const isDesktop = useIsDesktopApp()
  const params: { [key: string]: string } = rest

  if (enable_notifications) params.enable_notifications = 'true'
  if (isDesktop) params.desktop_app = 'true'

  return `${RAILS_AUTH_URL}/integrations/auth/new?${new URLSearchParams(params).toString()}`
}
