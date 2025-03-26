import { Link } from '@gitmono/ui/Link'
import { ConditionalWrap } from '@gitmono/ui/utils'

interface Props {
  action: string
  name: string
  href?: string | false
}

export function DesktopPermissionsDescription({ action, name, href }: Props) {
  return (
    <>
      To {action},{' '}
      <ConditionalWrap
        condition={!!href}
        wrap={(children) => (
          <Link target='_blank' className='text-blue-500 hover:underline' href={href || ''}>
            {children}
          </Link>
        )}
      >
        <>navigate to your system privacy settings</>
      </ConditionalWrap>{' '}
      and enable “{name}” permissions.
    </>
  )
}
