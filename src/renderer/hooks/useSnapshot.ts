import { useQuery, useQueryClient, type UseQueryResult } from '@tanstack/react-query'
import { startTransition, useEffect } from 'react'
import type { AppSnapshot } from '../../shared'

export const SNAPSHOT_QUERY_KEY = ['snapshot'] as const

/** The `getSnapshot` reply and `snapshot-changed` events can arrive in either order. */
function latest(incoming: AppSnapshot, held: AppSnapshot | undefined): AppSnapshot {
  return held && held.seq >= incoming.seq ? held : incoming
}

export function useSnapshot(): UseQueryResult<AppSnapshot> {
  const queryClient = useQueryClient()

  const query = useQuery<AppSnapshot>({
    queryKey: SNAPSHOT_QUERY_KEY,
    queryFn: async () => {
      const fetched = await window.easyVirtualDisplay.getSnapshot()
      return latest(fetched, queryClient.getQueryData<AppSnapshot>(SNAPSHOT_QUERY_KEY))
    },
    staleTime: Infinity
  })

  useEffect(() => {
    const unsubscribe = window.easyVirtualDisplay.subscribeSnapshot((snapshot) => {
      startTransition(() => {
        queryClient.setQueryData<AppSnapshot>(SNAPSHOT_QUERY_KEY, (held) =>
          latest(snapshot, held)
        )
      })
    })
    return unsubscribe
  }, [queryClient])

  return query
}
