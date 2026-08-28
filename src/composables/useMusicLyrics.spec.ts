import { afterEach, describe, expect, it, vi } from 'vitest'
import { tauriCommandMock } from '../tests/tauriBridgeMock'
import { useMusicLyrics } from './useMusicLyrics'

afterEach(() => {
  vi.useRealTimers()
})

describe('useMusicLyrics', () => {
  it('keeps waiting for a slow online lyrics response beyond the local timeout', async () => {
    vi.useFakeTimers()
    let resolveLyrics: (value: {
      content: string
      lrcContent: string
      source: string
      warnings: string[]
    }) => void = () => {
      throw new Error('Lyrics resolver was not initialized')
    }
    tauriCommandMock.mockImplementation((command: string) => {
      if (command !== 'read_kugou_lyrics') {
        throw new Error(`Unexpected lyrics command: ${command}`)
      }

      return new Promise((resolve) => {
        resolveLyrics = resolve
      })
    })

    const lyrics = useMusicLyrics()
    const loading = lyrics.loadLyricsForTrack({
      id: `kugou:${'D'.repeat(32)}`,
      path: `kugou:${'D'.repeat(32)}`,
      title: '慢速在线歌词测试',
      artist: '测试歌手',
      duration: 180,
      source: 'kugou',
      kugouSongHash: 'D'.repeat(32),
    })

    await vi.advanceTimersByTimeAsync(15_100)
    expect(lyrics.lyricsStatus.value).toBe('loading')

    resolveLyrics({
      content: '[00:00.00]慢速在线歌词已加载',
      lrcContent: '[00:00.00]慢速在线歌词已加载',
      source: 'kugou-lrc',
      warnings: [],
    })
    await loading

    expect(lyrics.lyricsStatus.value).toBe('ready')
    expect(lyrics.lyricsAt(1, {
      id: `kugou:${'D'.repeat(32)}`,
      path: `kugou:${'D'.repeat(32)}`,
      title: '慢速在线歌词测试',
      artist: '测试歌手',
      duration: 180,
      source: 'kugou',
      kugouSongHash: 'D'.repeat(32),
    }).current).toBe('慢速在线歌词已加载')
    expect(tauriCommandMock).toHaveBeenCalledTimes(1)
  })
})
