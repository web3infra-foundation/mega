import { describe, expect, it } from 'vitest'

import { parseGitHubRepoUrl } from '../syncUtils'

describe('parseGitHubRepoUrl', () => {
  it('parses https github urls', () => {
    expect(parseGitHubRepoUrl('https://github.com/facebook/react')).toEqual({
      owner: 'facebook',
      repo: 'react'
    })
  })

  it('parses urls with .git suffix', () => {
    expect(parseGitHubRepoUrl('https://github.com/facebook/react.git')).toEqual({
      owner: 'facebook',
      repo: 'react'
    })
  })

  it('parses host-only urls without protocol', () => {
    expect(parseGitHubRepoUrl('github.com/facebook/react')).toEqual({
      owner: 'facebook',
      repo: 'react'
    })
  })

  it('parses owner/repo shorthand', () => {
    expect(parseGitHubRepoUrl('facebook/react')).toEqual({
      owner: 'facebook',
      repo: 'react'
    })
  })

  it('rejects invalid hosts and malformed input', () => {
    expect(parseGitHubRepoUrl('https://gitlab.com/foo/bar')).toBeNull()
    expect(parseGitHubRepoUrl('https://github.com/facebook')).toBeNull()
    expect(parseGitHubRepoUrl('not a url')).toBeNull()
  })
})
