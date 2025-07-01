// import { MoreOutlined } from '@ant-design/icons';
import { NotePlusIcon } from '@gitmono/ui/Icons'
import type { MenuProps } from 'antd'
import { Card, Dropdown } from 'antd/lib'
import { formatDistance, fromUnixTime } from 'date-fns'
import { getMarkdownExtensions } from '@gitmono/editor'
import { useDeleteIssueComment } from '@/hooks/issues/useDeleteIssueComment'
import { useDeleteMrCommentDelete } from '@/hooks/useDeleteMrCommentDelete'
import { Conversation } from '@/pages/[org]/mr/[id]'
import { RichTextRenderer } from '@/components/RichTextRenderer'
import { useMemo } from 'react'
interface CommentProps {
  conv: Conversation
  id: string
  whoamI: string
}

const Comment = ({ conv, id, whoamI }: CommentProps) => {
  const { mutate: deleteComment } = useDeleteMrCommentDelete(id)
  const { mutate: deleteIssueComment } = useDeleteIssueComment(id)
  const handleDelete = () => {
    switch (whoamI) {
      case 'issue':
        deleteIssueComment(conv.id)
        break
      case 'mr':
        deleteComment(conv.id)
        break
      default:
        return
    }
  }

  const handleMenuClick: MenuProps['onClick'] = ({ key }) => {
    if (key === '3') {
      handleDelete()
    }
  }

  const items: MenuProps['items'] = [
    {
      label: 'Edit',
      key: '1',
      disabled: true
    },
    {
      label: 'Hide',
      key: '2',
      disabled: true
    },
    {
      type: 'divider'
    },
    {
      label: 'Delete',
      key: '3',
      danger: true
    }
  ]
  const menuProps = {
    items,
    onClick: handleMenuClick
  }

  const time = formatDistance(fromUnixTime(conv.created_at), new Date(), { addSuffix: true })
  const extensions = useMemo(() => getMarkdownExtensions({ linkUnfurl: {} }), [])

  return (
    <Card
      size='small'
      title={'Mega commented ' + time}
      style={{ border: '1px solid #d1d9e0' }}
      extra={
        <Dropdown menu={menuProps} trigger={['click']}>
          <NotePlusIcon />
        </Dropdown>
      }
    >
      <div className='prose copyable-text'>
        <RichTextRenderer content={conv.comment} extensions={extensions} />
      </div>
    </Card>
  )
}

export default Comment
