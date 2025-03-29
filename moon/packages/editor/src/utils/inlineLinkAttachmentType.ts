import { figmaRegex } from '@gitmono/regex'

/**
 * Returns supported 3rd party platforms for inline link attachments.
 */
export function inlineLinkAttachmentType(url: string) {
  if (figmaRegex.test(url)) return 'figma'

  return null
}
