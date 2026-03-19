import * as dotenv from 'dotenv'

const NODE_ENV = process.env.NODE_ENV || 'development'

// In non-production environments, load variables from .env.local for local development.
if (NODE_ENV !== 'production') {
  dotenv.config({ path: '.env.local' })
}

export const API_URL = process.env.API_URL
export const PORT = parseInt(process.env.PORT || '9000', 10)
export const IS_PRODUCTION = NODE_ENV === 'production'
