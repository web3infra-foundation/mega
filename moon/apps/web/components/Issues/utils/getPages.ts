export const getPages = (current: number, total: number): (number | '...')[] => {
  const maxVisible = 11
  const pages: (number | '...')[] = []

  // lower than maxVisible show them all
  if (total < maxVisible) return [...Array(total)].map((_, i) => i + 1)

  const left = Math.max(1, current - 2)
  const right = Math.min(total, current + 2)

  // left & right show two
  if (left > 2) {
    pages.push(1, 2, '...')
  } else {
    for (let i = 1; i < left; i++) pages.push(i)
  }

  // middle part
  for (let i = left; i <= right; i++) {
    pages.push(i)
  }

  if (right < total - 2) {
    pages.push('...', total - 1, total)
  } else {
    for (let i = right + 1; i <= total; i++) pages.push(i)
  }
  return pages
}
