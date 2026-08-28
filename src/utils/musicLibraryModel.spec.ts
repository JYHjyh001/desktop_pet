import { describe, expect, it } from 'vitest'
import {
  normalizeMusicPlaylist,
  normalizePlaylistName,
  normalizeQueueIds,
  normalizeTrackDate,
  normalizeValidTrackIds,
} from './musicLibraryModel'

describe('musicLibraryModel', () => {
  it('normalizes queue ids without applying library membership rules', () => {
    expect(normalizeQueueIds([' first ', '', 'second', 'first'])).toEqual([
      'first',
      'second',
      'first',
    ])
  })

  it('keeps only unique ids that still exist in the local library', () => {
    const validTrackIds = new Set(['first', 'second'])

    expect(normalizeValidTrackIds([' first ', 'missing', 'second', 'first'], validTrackIds)).toEqual([
      'first',
      'second',
    ])
  })

  it('normalizes persisted playlists with a deterministic fallback timestamp', () => {
    const fallbackNow = '2026-08-28T00:00:00.000Z'

    expect(
      normalizeMusicPlaylist(
        {
          id: ' playlist-1 ',
          name: ` ${'歌'.repeat(40)} `,
          trackIds: ['track-1', 'missing', 'track-1'],
          createdAt: 'invalid',
          updatedAt: '2026-08-27T00:00:00.000Z',
        },
        new Set(['track-1']),
        fallbackNow,
      ),
    ).toEqual({
      id: 'playlist-1',
      name: '歌'.repeat(32),
      trackIds: ['track-1'],
      createdAt: fallbackNow,
      updatedAt: '2026-08-27T00:00:00.000Z',
    })
  })

  it('rejects incomplete playlists and invalid dates', () => {
    expect(normalizeMusicPlaylist({ id: '', name: '列表' }, new Set())).toBeNull()
    expect(normalizeMusicPlaylist({ id: 'id', name: '  ' }, new Set())).toBeNull()
    expect(normalizeTrackDate('not-a-date')).toBeNull()
    expect(normalizePlaylistName(` ${'a'.repeat(40)} `)).toHaveLength(32)
  })
})
