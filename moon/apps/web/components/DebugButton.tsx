import { Button } from '@gitmono/ui/Button'

export function DebugButton() {
  return (
    <Button
      onClick={() => {
        throw new Error('Throw Exception Test 💥')
      }}
    >
      Throw from @components
    </Button>
  )
}
