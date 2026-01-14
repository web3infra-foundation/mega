declare module '@pierre/diffs' {
  export type ChangeTypes = 'change' | 'rename-pure' | 'rename-changed' | 'new' | 'deleted'

  export interface FileDiffHunk {
    header: string
    additionCount: number
    deletionCount: number
    lines: Array<{
      type: 'addition' | 'deletion' | 'context'
      content: string
      oldLineNumber?: number
      newLineNumber?: number
    }>
  }

  export interface FileDiffMetadata {
    name: string
    oldName?: string
    lang?: string
    type?: ChangeTypes
    hunks: FileDiffHunk[]
  }

  export interface PatchFile {
    files: FileDiffMetadata[]
  }

  export function parsePatchFiles(patch: string): PatchFile[]
}

declare module '@pierre/diffs/react' {
  import { CSSProperties, ComponentType } from 'react'
  import { FileDiffMetadata } from '@pierre/diffs'

  export interface FileDiffOptions {
    theme?: { dark: string; light: string } | string
    diffStyle?: 'unified' | 'split'
    diffIndicators?: 'classic' | 'modern'
    overflow?: 'wrap' | 'scroll'
    disableFileHeader?: boolean
    unsafeCSS?: string
  }

  export interface FileDiffProps {
    fileDiff: FileDiffMetadata
    options?: FileDiffOptions
    style?: CSSProperties
    className?: string
  }

  export const FileDiff: ComponentType<FileDiffProps>
}
