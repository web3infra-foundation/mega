import { Link } from '@gitmono/ui/Link'
import { NodeHandler } from '.'
import { useScope } from '@/contexts/scope'

export const LinkIssue: NodeHandler = ({ node, children }) => {
  const { scope } = useScope()

  if (!node.attrs || !node.attrs.id) {
    return <span>{children}</span>
  }

  const issueId = node.attrs?.id
  const label = node.attrs?.label || issueId
  const issueUrl = `/${scope}/issue/${issueId}`

  return (
    <Link data-type='issue' href={issueUrl} className="issue-link">
      <span className='text-blue-500 border-b border-blue-500'>${label}</span>
    </Link>
  )
}