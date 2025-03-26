import { MessageThread } from '@gitmono/types'
import { LayeredHotkeys } from '@gitmono/ui/DismissibleLayer'

import { FavoriteButton } from '@/components/FavoriteButton'
import { useCreateThreadFavorite } from '@/hooks/useCreateThreadFavorite'
import { useDeleteThreadFavorite } from '@/hooks/useDeleteThreadFavorite'

export function ChatFavoriteButton({
  thread,
  shortcutEnabled = false
}: {
  thread: MessageThread
  shortcutEnabled?: boolean
}) {
  const { mutate: createFavorite, isPending: isCreatePending } = useCreateThreadFavorite()
  const { mutate: deleteFavorite, isPending: isDeletePending } = useDeleteThreadFavorite()
  const isPending = isCreatePending || isDeletePending

  return (
    <>
      {shortcutEnabled && (
        <LayeredHotkeys
          keys='alt+f'
          callback={() => {
            if (thread.viewer_has_favorited) {
              deleteFavorite(thread.id)
            } else {
              createFavorite(thread)
            }
          }}
        />
      )}

      <FavoriteButton
        hasFavorited={thread.viewer_has_favorited}
        onFavorite={() => createFavorite(thread)}
        onRemoveFavorite={() => deleteFavorite(thread.id)}
        disabled={isPending}
        shortcutEnabled={shortcutEnabled}
      />
    </>
  )
}
