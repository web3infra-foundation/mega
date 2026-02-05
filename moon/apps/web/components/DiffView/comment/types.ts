import type { AnnotationSide } from '@pierre/diffs'

export interface CommentMetadata {
  id: string
  author: string
  avatarUrl: string
  content: string
  timestamp: string
  side: AnnotationSide
  lineNumber: number
  replies?: CommentMetadata[]
  isResolved?: boolean
}

export interface CommentThreadMetadata extends CommentMetadata {
  isThread: true
}

export interface CommentFormMetadata {
  isThread: false
}

export type AnnotationMetadata = CommentThreadMetadata | CommentFormMetadata
