import toast from 'react-hot-toast'

import { ApiError, ApiErrorTypes } from '@gitmono/types'

export function apiErrorToast(error: Error) {
  // never toast when there are connection stability errors
  if (error instanceof ApiError && error.name === ApiErrorTypes.ConnectionError) {
    return
  }
  toast.error(error.message)
}
