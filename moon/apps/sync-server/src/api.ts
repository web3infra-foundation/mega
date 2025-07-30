import { Api } from '@gitmono/types'

let baseUrl = 'https://api.gitmega.com'

if (process.env.NODE_ENV === 'production') {
  baseUrl = 'https://api.gitmega.com'
}

export const api = new Api({
  baseUrl,
  baseApiParams: {
    headers: { 'Content-Type': 'application/json' },
    format: 'json'
  }
})
