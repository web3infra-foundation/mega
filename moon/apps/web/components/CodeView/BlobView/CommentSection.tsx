import React, { useState } from 'react'

import { Avatar } from '@gitmono/ui/Avatar'

import { CommentEditor } from './CommentEditor'
import { CommentItem } from './CommentItem'

interface Comment {
  id: string
  content: string
  author: {
    id: string
    name: string
    avatar?: string
  }
  createdAt: Date
  replies?: Comment[]
}

interface CommentSectionProps {
  comments: Comment[]
  currentUser?: {
    id: string
    name: string
    avatar?: string
  }
  onAddComment: (content: string) => Promise<void>
  onReplyComment: (parentId: string, content: string) => Promise<void>
  onEditComment?: (commentId: string, content: string) => Promise<void>
  onDeleteComment?: (commentId: string) => Promise<void>
  isLoading?: boolean
}

export function CommentSection({
  comments,
  currentUser,
  onAddComment,
  onReplyComment,
  onEditComment,
  onDeleteComment,
  isLoading = false
}: CommentSectionProps) {
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [replyingTo, setReplyingTo] = useState<string | null>(null)
  const [__editCommentId, setEditCommentId] = useState<string | null>(null)

  const handleSubmitComment = async (content: string) => {
    if (!currentUser) return

    setIsSubmitting(true)
    try {
      await onAddComment(content)
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleReply = async (parentId: string, content: string) => {
    if (!currentUser) return

    setIsSubmitting(true)
    try {
      await onReplyComment(parentId, content)
      setReplyingTo(null)
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleEditComment = async (commentId: string, content: string) => {
    if (!currentUser) return

    setIsSubmitting(true)
    try {
      if (onEditComment) {
        await onEditComment(commentId, content)
      }
      setEditCommentId(null)
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <div className='space-y-6'>
      {currentUser && (
        <div className='flex space-x-3'>
          <Avatar src={currentUser.avatar} alt={currentUser.name} size='sm' />
          <div className='flex-1'>
            <CommentEditor onSubmit={handleSubmitComment} isSubmitting={isSubmitting} placeholder='参与讨论...' />
          </div>
        </div>
      )}

      {/*the comment modification function has not been implemented yet*/}
      <div className='space-y-6'>
        {comments.map((comment) => (
          <div key={comment.id}>
            <CommentItem
              {...comment}
              onReply={(id) => setReplyingTo(id)}
              onEdit={(id) => handleEditComment(id, comment.content)}
              onDelete={onDeleteComment}
              canEdit={currentUser?.id === comment.author.id}
              canDelete={currentUser?.id === comment.author.id}
            />

            {replyingTo === comment.id && currentUser && (
              <div className='ml-12 mt-3'>
                <CommentEditor
                  onSubmit={(content) => handleReply(comment.id, content)}
                  onCancel={() => setReplyingTo(null)}
                  isSubmitting={isSubmitting}
                  placeholder={`回复 @${comment.author.name}...`}
                />
              </div>
            )}

            {comment.replies && comment.replies.length > 0 && (
              <div className='ml-12 mt-4 space-y-4'>
                {comment.replies.map((reply) => (
                  <CommentItem
                    key={reply.id}
                    {...reply}
                    canEdit={currentUser?.id === reply.author.id}
                    canDelete={currentUser?.id === reply.author.id}
                  />
                ))}
              </div>
            )}
          </div>
        ))}
      </div>

      {isLoading && (
        <div className='py-4 text-center'>
          <span className='text-tertiary'>加载中...</span>
        </div>
      )}
    </div>
  )
}
