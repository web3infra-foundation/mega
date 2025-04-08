import { useEffect } from 'react'
import Router from 'next/router'

import { Post } from '@gitmono/types/generated'
import { ToastWithLink } from '@gitmono/ui/Toast'

import { useScope } from '@/contexts/scope'

export function PostComposerNewPostToast({ post }: { post: Post }) {
  const { scope } = useScope()

  useEffect(() => {
    Router.prefetch(`/${scope}/posts/${post.id}`)
  }, [scope, post.id])

  return <ToastWithLink url={post.url}>Post created</ToastWithLink>
}
