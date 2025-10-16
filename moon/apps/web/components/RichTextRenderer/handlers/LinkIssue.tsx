import { Link } from '@gitmono/ui/Link'
import { NodeHandler } from '.'
import { useScope } from '@/contexts/scope'

export const LinkIssue: NodeHandler = ({ node, children }) => {
  const { scope } = useScope()

  if (!node.attrs || !node.attrs.id) {
    return <span>{children}</span>
  }

  const id = node.attrs?.id
  const label = node.attrs?.label || id
  let url = '/404'

  switch (node.attrs?.suggestionType) {
    case 'issue':
      url = `/${scope}/issue/${id}`
      break
    case 'change_list':
      url = `/${scope}/cl/${id}`
      break
    default:
      break
  }

  return (
    <Link data-type='issue' href={url} className="issue-link">
      <span className='text-blue-500 border-b border-blue-500'>${label}</span>
    </Link>
  )
}