import type { AnchorHTMLAttributes, MouseEvent } from 'react'
import { pushUrl, replaceUrl } from '../../shared/navigation/history'

type AdminLinkProps = AnchorHTMLAttributes<HTMLAnchorElement> & {
  href: string
  replace?: boolean
}

function shouldUseClientNavigation(event: MouseEvent<HTMLAnchorElement>) {
  return event.button === 0
    && !event.defaultPrevented
    && !event.metaKey
    && !event.altKey
    && !event.ctrlKey
    && !event.shiftKey
    && !event.currentTarget.target
}

export default function AdminLink({ href, replace = false, onClick, children, ...props }: AdminLinkProps) {
  return (
    <a
      {...props}
      href={href}
      onClick={(event) => {
        onClick?.(event)
        if (!shouldUseClientNavigation(event)) return
        event.preventDefault()
        if (replace) {
          replaceUrl(href)
          return
        }
        pushUrl(href)
      }}
    >
      {children}
    </a>
  )
}
