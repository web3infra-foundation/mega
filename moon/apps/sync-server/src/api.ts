import { Api } from '@gitmono/types'

import { API_URL } from './config'

// Main API client (used for note sync)
export const api = new Api({
  baseUrl: API_URL,
  baseApiParams: {
    headers: { 'Content-Type': 'application/json' },
    format: 'json'
  }
})
