import { Colord, colord } from 'colord'

export function getFontColor(color: string): Colord {
  const isDark = colord(color).isDark()
  let fontColor: Colord = colord(color)

  if(isDark) fontColor = fontColor.lighten(0.6)
  else fontColor = fontColor.darken(0.5)

  return fontColor
}