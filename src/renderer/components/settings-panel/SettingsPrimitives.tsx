import { Switch } from '../ui/switch'
import { Button } from '../ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../ui/select'
import { Shield, Plus, Trash2, Save } from 'lucide-react'

export { Switch, Button, Select, SelectContent, SelectItem, SelectTrigger, SelectValue }
export { Shield, Plus, Trash2, Save }

export function SectionTitle({ children }: { children: React.ReactNode }): React.JSX.Element {
  return (
    <div className="flex items-center gap-2 mb-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
        {children}
      </h3>
      <div className="flex-1 h-px bg-border" />
    </div>
  )
}

export function SettingRow({
  label,
  description,
  children
}: {
  label: string
  description?: string
  children: React.ReactNode
}): React.JSX.Element {
  return (
    <div className="flex items-center justify-between gap-4 py-2.5">
      <div className="flex flex-col gap-0.5">
        <span className="text-sm text-foreground">{label}</span>
        {description && <span className="text-xs text-muted-foreground">{description}</span>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}
