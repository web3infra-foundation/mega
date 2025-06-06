import { Api } from '@gitmono/types'

let baseUrl = 'http://api.gitmega.com'

if (process.env.NODE_ENV === 'production') {
  baseUrl = 'http://api.gitmega.com'
}

export const api = new Api({
  baseUrl,
  baseApiParams: {
    headers: { 'Content-Type': 'application/json' },
    format: 'json'
  }
})
