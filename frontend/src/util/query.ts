import {useMemo} from "react";

export function useQuery() {
    return useMemo(
        () => new URLSearchParams(typeof window === 'undefined' ? '' : window.location.search),
        [],
    )
}
