export interface MusicPlaylist {
  id: string
  name: string
  trackIds: string[]
  createdAt: string
  updatedAt: string
}

export interface StoredMusicPlaylist {
  id?: string
  name?: string
  trackIds?: string[]
  createdAt?: string
  updatedAt?: string
}

export function normalizeTrackId(value?: string | null) {
  return value?.trim() ?? ''
}

export function normalizeTrackDate(value?: string | null) {
  if (!value || Number.isNaN(Date.parse(value))) {
    return null
  }

  return value
}

export function normalizePlaylistName(value?: string | null) {
  return (value?.trim() ?? '').slice(0, 32)
}

export function normalizeQueueIds(value?: string[]) {
  if (!Array.isArray(value)) {
    return []
  }

  return value.map(normalizeTrackId).filter(Boolean)
}

export function normalizeValidTrackIds(value: string[] | undefined, validTrackIds: ReadonlySet<string>) {
  const seenTrackIds = new Set<string>()

  return normalizeQueueIds(value).filter((trackId) => {
    if (!validTrackIds.has(trackId) || seenTrackIds.has(trackId)) {
      return false
    }

    seenTrackIds.add(trackId)
    return true
  })
}

export function normalizeMusicPlaylist(
  playlist: StoredMusicPlaylist,
  validTrackIds: ReadonlySet<string>,
  now = new Date().toISOString(),
): MusicPlaylist | null {
  const id = normalizeTrackId(playlist.id)
  const name = normalizePlaylistName(playlist.name)
  if (!id || !name) {
    return null
  }

  return {
    id,
    name,
    trackIds: normalizeValidTrackIds(playlist.trackIds, validTrackIds),
    createdAt: normalizeTrackDate(playlist.createdAt) || now,
    updatedAt: normalizeTrackDate(playlist.updatedAt) || now,
  }
}
