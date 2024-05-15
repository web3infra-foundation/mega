import SVG from 'react-inlinesvg';
import Image from "next/image";

import logoMega from '@/images/logo.svg';


export function LogoMark(props) {
  return (
      <SVG src="@/images/logo.svg" {...props} />
  )
}

export function Logo(props) {
  return (
      <Image src={logoMega} alt="Mega" unoptimized className="h-20 w-auto ml-0"/>
  )
}
