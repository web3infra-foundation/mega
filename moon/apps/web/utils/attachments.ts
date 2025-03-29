import { Attachment } from '@gitmono/types'

export function isRenderable(attachment: Attachment) {
  return attachment.image || attachment.gif || attachment.video || attachment.lottie || attachment.link
}
