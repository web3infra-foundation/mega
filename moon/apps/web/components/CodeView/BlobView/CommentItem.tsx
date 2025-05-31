import React from 'react'
import { Avatar } from '@gitmono/ui/Avatar'
import { formatDistanceToNow } from 'date-fns'
import { zhCN } from 'date-fns/locale'

interface CommentItemProps {
  id: string
  content: string
  author: {
    id: string
    name: string
    avatar?: string
  }
  createdAt: Date
  onReply?: (commentId: string) => void
  onEdit?: (commentId: string) => void
  onDelete?: (commentId: string) => void
  canEdit?: boolean
  canDelete?: boolean
}

export function CommentItem({
                              id,
                              content,
                              author,
                              createdAt,
                              onReply,
                              onEdit,
                              onDelete,
                              canEdit = false,
                              canDelete = false
                            }: CommentItemProps) {

  return (
    <div className="flex space-x-3 py-4">
      <Avatar src={author.avatar} alt={author.name} size="sm" />

      <div className="flex-1 min-w-0">
        <div className="flex items-center space-x-2 mb-2">
          <span className="font-medium text-gray-900">{author.name}</span>
          <span className="text-xs text-gray-500">
            {formatDistanceToNow(createdAt, {
              addSuffix: true,
              locale: zhCN
            })}
          </span>
        </div>

        <div className="prose prose-sm max-w-none">
          <span className="font-medium text-gray-800">{content}</span>
        </div>

        <div className="flex items-center space-x-4 mt-3">
          {onReply && (
            <button
              onClick={() => onReply(id)}
              className="text-xs text-gray-500 hover:text-gray-700"
            >
              回复
            </button>
          )}

          {canEdit && onEdit && (
            <button
              onClick={() => onEdit(id)}
              className="text-xs text-gray-500 hover:text-gray-700"
            >
              编辑
            </button>
          )}

          {canDelete && onDelete && (
            <button
              onClick={() => onDelete(id)}
              className="text-xs text-red-500 hover:text-red-700"
            >
              删除
            </button>
          )}
        </div>
      </div>
    </div>
  )
}