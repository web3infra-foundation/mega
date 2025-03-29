import { Attachment } from '@gitmono/types'

export const stableId = (attachment: Attachment) => attachment.optimistic_id ?? attachment.id
