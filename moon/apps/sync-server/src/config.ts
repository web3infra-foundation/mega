import * as path from 'path'
import * as dotenv from 'dotenv'

// Load environment-specific .env file
// Priority: .env.{NODE_ENV} > .env.local > .env
const envFile = process.env.NODE_ENV ? `.env.${process.env.NODE_ENV}` : '.env.local'

dotenv.config({ path: path.resolve(process.cwd(), envFile) })

// Fallback to .env.local if environment-specific file doesn't exist
if (process.env.NODE_ENV && !process.env.API_URL) {
  dotenv.config({ path: path.resolve(process.cwd(), '.env.local') })
}

// API URL - read from environment variable with fallback default
export const API_URL = process.env.API_URL || 'https://api.gitmega.com'

// Server Configuration
export const PORT = parseInt(process.env.PORT || '9000', 10)
export const NODE_ENV = process.env.NODE_ENV || 'development'
export const IS_PRODUCTION = NODE_ENV === 'production'
