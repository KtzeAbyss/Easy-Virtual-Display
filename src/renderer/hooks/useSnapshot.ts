import { useQuery, useQueryClient, type UseQueryResult } from '@tanstack/react-query'
import { startTransition, useEffect } from 'react'
import type { AppSnapshot } from '../../shared'

export const SNAPSHOT_QUERY_KEY = ['snapshot'] as const

export function useSnapshot(): UseQueryResult<AppSnapshot> {
  const queryClient = useQueryClient()

  const query = useQuery<AppSnapshot>({
    queryKey: SNAPSHOT_QUERY_KEY,
    queryFn: () => window.easyVirtualDisplay.getSnapshot(),
    staleTime: Infinity
  })

  useEffect(() => {
    const unsubscribe = window.easyVirtualDisplay.subscribeSnapshot((snapshot) => {
      startTransition(() => {
        queryClient.setQueryData(SNAPSHOT_QUERY_KEY, snapshot)
      })
    })
    return unsubscribe
  }, [queryClient])

  return query
}
