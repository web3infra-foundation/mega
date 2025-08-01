import { mergeAttributes, Node, Range } from '@tiptap/core'

import { handleRestoreMentionBackspace } from '../utils/handleRestoreMentionBackspace'
import { insertContent } from '../utils/insertContent'

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    issue: {
      insertIssue: (props: {
        id: string
        label: string
        suggestionType: string
        range: Range
      }) => ReturnType
    }
  }
}

export interface IssueNodeAttrs {
  /**
   * The identifier for the selected issue, stored as a `data-id` attribute.
   */
  id: string | null
  /**
   * The label to be rendered by the editor as the displayed text for this issue.
   */
  label?: string | null
}

export type IssueOptions = {
  HTMLAttributes: Record<string, any>
}

export const LinkIssue = Node.create<IssueOptions>({
  name: 'linkIssue',

  addOptions() {
    return {
      HTMLAttributes: {
        class: 'link-issue'
      }
    }
  },

  group: 'inline',

  inline: true,

  selectable: false,

  atom: true,

  addAttributes() {
    return {
      id: {
        default: null,
        parseHTML: (element) => element.getAttribute('data-id'),
        renderHTML: (attributes) => {
          if (!attributes.id) {
            return {}
          }

          return {
            'data-id': attributes.id
          }
        }
      },

      label: {
        default: null,
        parseHTML: (element) => element.getAttribute('data-label'),
        renderHTML: (attributes) => {
          if (!attributes.label) {
            return {}
          }

          return {
            'data-label': attributes.label
          }
        }
      },

      suggestionType: {
        default: null,
        parseHTML: (element) => element.getAttribute('data-suggestionType'),
        renderHTML: (attributes) => {
          if (!attributes.suggestionType) {
            return {}
          }

          return {
            'data-suggestionType': attributes.suggestionType
          }
        }
      }
    }
  },

  parseHTML() {
    return [
      {
        tag: `span[data-type="${this.name}"]`
      }
    ]
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'span',
      mergeAttributes(this.options.HTMLAttributes, { 'data-type': this.name }, HTMLAttributes),
      `$${node.attrs.label ?? node.attrs.id}`
    ]
  },

  renderText({ node }) {
    return `$${node.attrs.label ?? node.attrs.id}`
  },

  addCommands() {
    return {
      insertIssue:
        ({ range, ...attrs }) =>
        ({ chain, state }) =>
          insertContent({
            chain,
            range,
            state,
            content: { type: this.name, attrs }
          })
    }
  },

  addKeyboardShortcuts() {
    return {
      Backspace: () =>
        this.editor.commands.command(({ tr, state }) =>
          handleRestoreMentionBackspace({ transaction: tr, state, nodeName: this.name, char: '$' })
        )
    }
  }
})