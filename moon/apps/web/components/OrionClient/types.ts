'use client'

export type OrionClientStatus =
  | 'idle'
  | 'busy'
  | 'running'
  | 'downloading'
  | 'preparing'
  | 'uploading'
  | 'error'
  | 'offline'

export interface OrionClient {
  client_id: string
  hostname: string
  instance_id: string
  orion_version: string
  start_time: string
  last_heartbeat: string
  supported_capabilities?: string | string[]
  status?: OrionClientStatus
}

export const OFFLINE_THRESHOLD_SECONDS = 30

export function deriveStatus(client: OrionClient, now: number = Date.now()): OrionClientStatus {
  if (client.status) return client.status

  const lastHeartbeat = new Date(client.last_heartbeat).getTime()
  const diffSeconds = (now - lastHeartbeat) / 1000

  if (Number.isFinite(diffSeconds) && diffSeconds > OFFLINE_THRESHOLD_SECONDS) {
    return 'offline'
  }

  return 'idle'
}
