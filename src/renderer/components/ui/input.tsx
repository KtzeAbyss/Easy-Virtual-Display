import * as React from 'react'

import { cn } from '@/lib/utils'

/** Fluent TextBox: control fill, accent underline while focused. */
function Input({ className, type, ...props }: React.ComponentProps<'input'>): React.JSX.Element {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        'h-8 w-full min-w-0 rounded-md border border-control-border border-b-muted-foreground/50 bg-control px-2.5 text-sm transition-colors outline-none placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground hover:bg-control-hover disabled:pointer-events-none disabled:opacity-40',
        'focus-visible:border-b-2 focus-visible:border-b-primary focus-visible:bg-popover',
        'aria-invalid:border-b-2 aria-invalid:border-b-status-error',
        className
      )}
      {...props}
    />
  )
}

export { Input }
