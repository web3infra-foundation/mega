import type { AnnotationSide, GetHoveredLineResult } from '@pierre/diffs'

interface HoverButtonProps {
  getHoveredLine: () => GetHoveredLineResult<'diff'> | undefined
  onAddComment: (side: AnnotationSide, lineNumber: number) => void
}

export function HoverButton({ getHoveredLine, onAddComment }: HoverButtonProps) {
  const handleClick = (event: React.MouseEvent) => {
    const hoveredLine = getHoveredLine()

    if (hoveredLine == null) return

    event.stopPropagation()

    onAddComment(hoveredLine.side, hoveredLine.lineNumber)
  }

  return (
    <button
      onClick={handleClick}
      className='flex h-5 w-5 items-center justify-center rounded bg-blue-500 text-white transition-all hover:bg-blue-700'
      style={{
        cursor: 'pointer'
      }}
    >
      <svg className='h-3 w-3' fill='none' stroke='currentColor' viewBox='0 0 24 24' xmlns='http://www.w3.org/2000/svg'>
        <path strokeLinecap='round' strokeLinejoin='round' strokeWidth={3} d='M12 4v16m8-8H4' />
      </svg>
    </button>
  )
}
