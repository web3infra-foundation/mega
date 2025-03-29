import { ResourceMentionOptions } from '@gitmono/editor'
import { Link } from '@gitmono/ui/Link'

import { ResourceMentionView } from '@/components/InlineResourceMentionRenderer'

import { NodeHandler } from '.'

export const InlineResourceMention: NodeHandler<ResourceMentionOptions> = ({ node }) => {
  const href = node.attrs?.href

  return (
    <Link href={href}>
      <ResourceMentionView href={href} />
    </Link>
  )
}
