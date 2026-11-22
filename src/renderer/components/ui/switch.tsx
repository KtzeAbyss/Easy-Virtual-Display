import * as React from 'react'
import * as SwitchPrimitive from '@radix-ui/react-switch'

import { cn } from '@/lib/utils'

/** Fluent ToggleSwitch: 40×20 track, outlined when off, accent-filled when on. */
function Switch({
  className,
  ...props
}: React.ComponentProps<typeof SwitchPrimitive.Root>): React.JSX.Element {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        'peer group inline-flex h-5 w-10 shrink-0 items-center rounded-full border px-[3px] transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background disabled:opacity-40',
        'data-[state=unchecked]:border-muted-foreground data-[state=unchecked]:bg-control data-[state=unchecked]:hover:bg-control-hover',
        'data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=checked]:hover:bg-primary/90',
        className
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className="pointer-events-none block size-3 rounded-full transition-transform group-active:scale-x-125 data-[state=unchecked]:translate-x-0 data-[state=unchecked]:bg-muted-foreground data-[state=checked]:translate-x-5 data-[state=checked]:bg-primary-foreground"
      />
    </SwitchPrimitive.Root>
  )
}

export { Switch }
