import { Link, LinkProps } from '@gitmono/ui/Link'
import { useScope } from '@/contexts/scope'

export function UserLinkByName({
  username,
  children,
  ...linkProps
}: Omit<LinkProps, 'href'> & { username: string; children: React.ReactNode }) {
  const { scope } = useScope()

  if(!username) return <>{children}</>

  return (
    <Link {...linkProps} href={`/${scope}/people/${username}`}>
      {children}
    </Link>
  )
}