import { flushPromises, shallowMount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { tauriCommandMock } from '../tests/tauriBridgeMock'
import MusicWindow from './MusicWindow.vue'

type TestTheme = 'light' | 'animal-island'

beforeEach(() => {
  const values = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    get length() {
      return values.size
    },
    clear: () => values.clear(),
    getItem: (key: string) => values.get(key) ?? null,
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    removeItem: (key: string) => values.delete(key),
    setItem: (key: string, value: string) => values.set(key, value),
  } satisfies Storage)
})

afterEach(() => {
  vi.unstubAllGlobals()
})

function musicLibraryState() {
  return {
    version: 1,
    settings: {},
    tracks: [
      {
        id: 'test-track',
        path: 'test-data/sample.mp3',
        sourcePath: 'test-data/sample.mp3',
        title: '测试歌曲',
        artist: '测试歌手',
        album: '测试专辑',
        category: '未分类',
        tags: [],
        duration: 180,
        favorite: false,
        playCount: 0,
        lastPlayedAt: null,
        playHistory: [],
      },
    ],
    playlists: [
      {
        id: 'test-playlist',
        name: '测试歌单',
        trackIds: [] as string[],
        createdAt: '2026-01-01T00:00:00.000Z',
        updatedAt: '2026-01-01T00:00:00.000Z',
      },
    ],
    savedAt: null,
  }
}

function installMusicCommands(theme: TestTheme, libraryState = musicLibraryState()) {
  tauriCommandMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    if (command === 'get_music_library_state') return libraryState
    if (command === 'get_music_library_storage_info') {
      return {
        path: 'test-data/music-library.json',
        version: 1,
        trackCount: 1,
        playlistCount: 1,
        savedAt: null,
      }
    }
    if (command === 'get_public_config') {
      return {
        drawer: { theme, musicImmersiveTheme: 'follow' },
        ai: { enabled: false, provider: 'custom', model: 'test-model' },
      }
    }
    if (command === 'get_netease_login_status' || command === 'get_kugou_login_status') {
      return { loggedIn: false, message: '未登录' }
    }
    if (command === 'read_music_metadata') {
      return {
        title: '标签歌曲',
        artist: '标签歌手',
        album: '标签专辑',
        duration: 181,
        source: 'metadata',
        confidence: 1,
        warnings: [],
      }
    }
    if (command === 'read_music_lyrics') {
      const path = typeof args?.path === 'string' ? args.path : ''
      return {
        content: path.includes('next')
          ? '[00:00.00]切歌歌词已加载\n[00:10.00]无需重新进入沉浸模式'
          : '[00:00.00]初始歌词已加载',
        source: 'test-lrc',
        warnings: [],
      }
    }
    if (command === 'save_music_library_state') return null

    throw new Error(`Unexpected Tauri command in music smoke test: ${command}`)
  })
}

async function openTrackActionMenu(wrapper: ReturnType<typeof shallowMount>) {
  const moreButton = wrapper.get<HTMLButtonElement>('button[aria-label="更多操作"]')
  moreButton.element.focus()
  await moreButton.trigger('click')
  return moreButton
}

describe('MusicWindow accessibility', () => {
  it.each(['light', 'animal-island'] as const)(
    'manages dialog semantics and focus in the %s theme',
    async (theme) => {
      installMusicCommands(theme)
      const wrapper = shallowMount(MusicWindow, {
        attachTo: document.body,
        global: {
          stubs: {
            MusicVisualizerCanvas: true,
            MusicWebglStarfield: true,
          },
        },
      })
      await flushPromises()

      expect(wrapper.get('.music-window').classes()).toContain(`theme-${theme}`)

      const settingsButton = wrapper.get<HTMLButtonElement>('button[title="打开音乐设置"]')
      settingsButton.element.focus()
      await settingsButton.trigger('click')
      await flushPromises()

      const settingsDialog = wrapper.get(
        '[role="dialog"][aria-labelledby="music-settings-title"]',
      )
      expect(settingsDialog.attributes('aria-describedby')).toBe('music-settings-description')
      expect(document.activeElement).toBe(
        settingsDialog.get<HTMLButtonElement>('[aria-label="关闭音乐设置"]').element,
      )

      await settingsDialog.trigger('keydown', { key: 'Escape' })
      await flushPromises()
      expect(wrapper.find('[aria-labelledby="music-settings-title"]').exists()).toBe(false)
      expect(document.activeElement).toBe(settingsButton.element)

      let moreButton = await openTrackActionMenu(wrapper)
      const editButton = wrapper.get<HTMLButtonElement>('button[aria-label="编辑歌名和歌手"]')
      editButton.element.focus()
      await editButton.trigger('click')
      await flushPromises()

      const editor = wrapper.get('[role="dialog"][aria-labelledby="music-track-editor-title"]')
      expect(editor.attributes('aria-describedby')).toBe('music-track-editor-description')
      expect(document.activeElement).toBe(editor.get('[data-dialog-initial-focus]').element)

      await editor.trigger('keydown', { key: 'Escape' })
      await flushPromises()
      expect(wrapper.find('[aria-labelledby="music-track-editor-title"]').exists()).toBe(false)
      expect(document.activeElement).toBe(moreButton.element)

      moreButton = await openTrackActionMenu(wrapper)
      const metadataButton = wrapper.get<HTMLButtonElement>(
        'button[aria-label="读取 metadata 标签"]',
      )
      metadataButton.element.focus()
      await metadataButton.trigger('click')
      await flushPromises()

      const metadataDialog = wrapper.get(
        '[role="dialog"][aria-labelledby="music-metadata-result-title"]',
      )
      expect(metadataDialog.attributes('aria-describedby')).toBe(
        'music-metadata-result-description',
      )
      expect(document.activeElement).toBe(
        metadataDialog.get('[data-dialog-initial-focus]').element,
      )

      await metadataDialog.trigger('keydown', { key: 'Escape' })
      await flushPromises()
      expect(wrapper.find('[aria-labelledby="music-metadata-result-title"]').exists()).toBe(false)
      expect(document.activeElement).toBe(moreButton.element)

      const addToPlaylistButton = wrapper
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().trim() === '加歌')
      expect(addToPlaylistButton).toBeDefined()
      addToPlaylistButton!.element.focus()
      await addToPlaylistButton!.trigger('click')
      await flushPromises()

      const picker = wrapper.get(
        '[role="dialog"][aria-labelledby="music-playlist-picker-title"]',
      )
      expect(picker.attributes('aria-describedby')).toBe('music-playlist-picker-description')
      expect(document.activeElement).toBe(picker.get('input').element)

      await picker.trigger('keydown', { key: 'Escape' })
      await flushPromises()
      expect(wrapper.find('[aria-labelledby="music-playlist-picker-title"]').exists()).toBe(false)
      expect(document.activeElement).toBe(addToPlaylistButton!.element)

      wrapper.unmount()
      await flushPromises()
    },
  )

  it.each(['light', 'animal-island'] as const)(
    'keeps the immersive playlist rack focused and single-expanded in the %s theme',
    async (theme) => {
      const libraryState = musicLibraryState()
      libraryState.playlists = [
        {
          ...libraryState.playlists[0],
          id: 'focus-playlist',
          name: '专注歌单',
          trackIds: ['test-track'],
        },
        {
          ...libraryState.playlists[0],
          id: 'rest-playlist',
          name: '休息歌单',
          trackIds: [],
        },
      ]
      installMusicCommands(theme, libraryState)
      vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue()

      const wrapper = shallowMount(MusicWindow, {
        attachTo: document.body,
        global: {
          stubs: {
            MusicVisualizerCanvas: true,
            MusicWebglStarfield: true,
          },
        },
      })
      await flushPromises()

      await wrapper.get('button[title="进入沉浸式音乐模式"]').trigger('click')
      await flushPromises()

      const playlistSourceTabs = wrapper.get('[aria-label="沉浸模式播放列表"]')
      const myPlaylistsTab = playlistSourceTabs
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().trim() === '我的歌单')
      expect(myPlaylistsTab).toBeDefined()
      await myPlaylistsTab!.trigger('click')

      const filterTabs = wrapper.get('[aria-label="筛选歌单来源"]')
      expect(filterTabs.findAll('button').map((button) => button.text().replace(/\s+/g, ''))).toEqual([
        '全部2',
        '本地2',
        '网易云0',
        '酷狗0',
      ])

      const localFilter = filterTabs
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().includes('本地'))
      expect(localFilter).toBeDefined()
      await localFilter!.trigger('click')

      expect(localFilter!.attributes('aria-selected')).toBe('true')
      expect(wrapper.find('[aria-label="沉浸模式网易云歌单"]').exists()).toBe(false)
      expect(wrapper.find('[aria-label="沉浸模式酷狗歌单"]').exists()).toBe(false)

      let playlistCards = wrapper
        .get('[aria-label="沉浸模式本地歌单"]')
        .findAll<HTMLButtonElement>('.music-immersive-playlist-row.playlist')
      expect(playlistCards).toHaveLength(2)

      await playlistCards[0].trigger('click')
      expect(
        wrapper.findAll('.music-immersive-playlist-detail-sticky'),
      ).toHaveLength(1)
      expect(wrapper.get('.music-immersive-playlist-detail-summary').text()).toContain('专注歌单')

      const playPlaylistButton = wrapper
        .get('.music-immersive-playlist-detail-sticky')
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().trim() === '播放歌单')
      expect(playPlaylistButton).toBeDefined()
      await playPlaylistButton!.trigger('click')
      await flushPromises()

      expect(wrapper.find('[aria-label="筛选歌单来源"]').exists()).toBe(true)
      expect(wrapper.get('.music-immersive-playlist-detail-summary').text()).toContain('专注歌单')

      playlistCards = wrapper
        .get('[aria-label="沉浸模式本地歌单"]')
        .findAll<HTMLButtonElement>('.music-immersive-playlist-row.playlist')
      await playlistCards[1].trigger('click')
      expect(
        wrapper.findAll('.music-immersive-playlist-detail-sticky'),
      ).toHaveLength(1)
      expect(wrapper.get('.music-immersive-playlist-detail-summary').text()).toContain('休息歌单')

      wrapper.unmount()
      await flushPromises()
    },
  )

  it.each(['light', 'animal-island'] as const)(
    'preserves the expanded Kugou playlist, scroll and active playback position in the %s theme',
    async (theme) => {
      installMusicCommands(theme)
      const playSpy = vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue()
      vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {})
      const fallbackCommand = tauriCommandMock.getMockImplementation()!
      const kugouPlaylists = [
        {
          id: 'focus-playlist',
          listId: 'focus-list',
          name: '专注歌单',
          trackCount: 1,
          subscribed: false,
        },
        {
          id: 'night-playlist',
          listId: 'night-list',
          name: '深夜歌单',
          trackCount: 2,
          subscribed: false,
        },
      ]
      let playlistRefreshCount = 0
      let playlistRackToReset: HTMLElement | null = null
      tauriCommandMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
        if (command === 'get_kugou_login_status') {
          return {
            loggedIn: true,
            profile: {
              userId: 'test-user',
              nickname: '测试账号',
              membership: null,
            },
            savedAt: null,
            checkedAt: '2026-01-01T00:00:00.000Z',
            message: '已登录',
          }
        }
        if (command === 'list_kugou_playlists') {
          playlistRefreshCount += 1
          if (playlistRefreshCount > 1 && playlistRackToReset) {
            playlistRackToReset.scrollTop = 0
          }
          return kugouPlaylists
        }
        if (command === 'get_kugou_playlist_detail') {
          if (playlistRackToReset) {
            playlistRackToReset.scrollTop = 0
          }
          const playlist =
            kugouPlaylists.find((item) => item.listId === args?.listId) ?? kugouPlaylists[0]
          const tracks = playlist.listId === 'night-list'
            ? [
                {
                  hash: 'B'.repeat(32),
                  name: '深夜测试歌曲',
                  artists: ['测试歌手'],
                  album: '测试专辑',
                  durationMs: 180000,
                },
                {
                  hash: 'C'.repeat(32),
                  name: '夜间第二首',
                  artists: ['测试歌手'],
                  album: '测试专辑',
                  durationMs: 210000,
                },
              ]
            : [
                {
                  hash: 'A'.repeat(32),
                  name: '专注测试歌曲',
                  artists: ['测试歌手'],
                  album: '测试专辑',
                  durationMs: 180000,
                },
              ]
          return {
            playlist,
            tracks,
            totalTrackCount: tracks.length,
            truncated: false,
            message: `已读取${playlist.name}`,
          }
        }
        if (command === 'get_kugou_song_quality_availability') {
          return {
            hash: args?.hash,
            qualities: [{ quality: 'standard', status: 'available' }],
            message: '标准音质可用',
          }
        }
        if (command === 'get_kugou_song_playback_url') {
          return {
            hash: args?.hash,
            url: 'https://example.test/kugou-audio.mp3',
            qualityLevel: 'standard',
            qualityLabel: '标准音质',
            bitrate: 128000,
            durationMs: 210000,
            fileType: 'mp3',
            size: 1024,
            message: '播放地址已就绪',
          }
        }
        if (command === 'read_kugou_lyrics') {
          return {
            songId: args?.hash,
            content: '[00:00.00]在线歌词无需重新进入沉浸模式',
            lrcContent: '[00:00.00]在线歌词无需重新进入沉浸模式',
            source: 'kugou-lrc',
            warnings: [],
          }
        }
        return fallbackCommand(command, args)
      })

      const wrapper = shallowMount(MusicWindow, {
        attachTo: document.body,
        global: {
          stubs: {
            MusicVisualizerCanvas: true,
            MusicWebglStarfield: true,
          },
        },
      })
      await flushPromises()

      await wrapper.get('button[title="进入沉浸式音乐模式"]').trigger('click')
      await flushPromises()
      const playlistSourceTabs = wrapper.get('[aria-label="沉浸模式播放列表"]')
      const myPlaylistsTab = playlistSourceTabs
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().trim() === '我的歌单')
      expect(myPlaylistsTab).toBeDefined()
      await myPlaylistsTab!.trigger('click')

      const filterTabs = wrapper.get('[aria-label="筛选歌单来源"]')
      const kugouFilter = filterTabs
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().includes('酷狗'))
      expect(kugouFilter).toBeDefined()
      await kugouFilter!.trigger('click')

      const kugouSection = () => wrapper.get('[aria-label="沉浸模式酷狗歌单"]')
      const refreshButton = () =>
        kugouSection()
          .findAll<HTMLButtonElement>('button')
          .find((button) => button.text().trim() === '刷新')
      expect(refreshButton()).toBeDefined()
      await refreshButton()!.trigger('click')
      await vi.waitFor(() => {
        expect(playlistRefreshCount).toBe(1)
        expect(kugouSection().findAll('.music-immersive-playlist-row.playlist')).toHaveLength(2)
        expect(refreshButton()!.attributes('disabled')).toBeUndefined()
      })

      const playlistCards = kugouSection().findAll<HTMLButtonElement>(
        '.music-immersive-playlist-row.playlist',
      )
      const playlistRack = wrapper.get<HTMLElement>('[aria-label="沉浸模式我的歌单"]')
      playlistRack.element.scrollTop = 132
      playlistRackToReset = playlistRack.element
      await playlistCards[1].trigger('click')
      await vi.waitFor(() => {
        expect(wrapper.get('.music-immersive-playlist-detail-summary').text()).toContain('深夜歌单')
        expect(
          wrapper.get<HTMLElement>('[aria-label="沉浸模式我的歌单"]').element.scrollTop,
        ).toBe(132)
        expect(refreshButton()!.attributes('disabled')).toBeUndefined()
      })

      playlistRack.element.scrollTop = 176
      await refreshButton()!.trigger('click')
      await vi.waitFor(() => {
        expect(playlistRefreshCount).toBe(2)
        expect(
          wrapper.get<HTMLElement>('[aria-label="沉浸模式我的歌单"]').element.scrollTop,
        ).toBe(176)
        expect(wrapper.get('.music-immersive-playlist-detail-summary').text()).toContain('深夜歌单')
      })

      const refreshedTracks = kugouSection().findAll<HTMLButtonElement>(
        '.music-immersive-playlist-row.compact',
      )
      expect(refreshedTracks).toHaveLength(2)
      await refreshedTracks[1].trigger('click')
      await vi.waitFor(() => {
        expect(
          kugouSection().findAll('.music-immersive-playlist-row.compact.active')[0]?.text(),
        ).toContain('夜间第二首')
        expect(wrapper.get('.music-immersive-lyrics-line.current').text()).toBe(
          '在线歌词无需重新进入沉浸模式',
        )
      })

      const audio = wrapper.get<HTMLAudioElement>('audio')
      audio.element.currentTime = 42
      await audio.trigger('timeupdate')
      const playCountBeforePlaylistResume = playSpy.mock.calls.length
      const playPlaylistButton = kugouSection()
        .get('.music-immersive-playlist-detail-sticky')
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().trim() === '播放歌单')
      expect(playPlaylistButton).toBeDefined()
      await playPlaylistButton!.trigger('click')
      await flushPromises()

      expect(audio.element.currentTime).toBe(42)
      expect(playSpy).toHaveBeenCalledTimes(playCountBeforePlaylistResume)
      expect(
        kugouSection().findAll('.music-immersive-playlist-row.compact.active')[0]?.text(),
      ).toContain('夜间第二首')
      expect(
        tauriCommandMock.mock.calls.filter(([command]) => command === 'read_kugou_lyrics'),
      ).toHaveLength(1)
      expect(wrapper.get('.music-window').classes()).toContain(`theme-${theme}`)

      wrapper.unmount()
      await flushPromises()
    },
  )

  it.each(['light', 'animal-island'] as const)(
    'loads the main lyrics immediately after switching tracks in immersive mode in the %s theme',
    async (theme) => {
      const libraryState = musicLibraryState()
      libraryState.tracks.push({
        ...libraryState.tracks[0],
        id: 'next-track',
        path: 'test-data/next.mp3',
        sourcePath: 'test-data/next.mp3',
        title: '切歌测试歌曲',
      })
      libraryState.playlists[0].trackIds = ['test-track', 'next-track']
      installMusicCommands(theme, libraryState)
      vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue()

      const wrapper = shallowMount(MusicWindow, {
        attachTo: document.body,
        global: {
          stubs: {
            MusicVisualizerCanvas: true,
            MusicWebglStarfield: true,
          },
        },
      })
      await flushPromises()

      await wrapper.get('button[title="进入沉浸式音乐模式"]').trigger('click')
      await flushPromises()

      const playlistSourceTabs = wrapper.get('[aria-label="沉浸模式播放列表"]')
      const myPlaylistsTab = playlistSourceTabs
        .findAll<HTMLButtonElement>('button')
        .find((button) => button.text().trim() === '我的歌单')
      expect(myPlaylistsTab).toBeDefined()
      await myPlaylistsTab!.trigger('click')

      const playlistCard = wrapper
        .get('[aria-label="沉浸模式本地歌单"]')
        .get<HTMLButtonElement>('.music-immersive-playlist-row.playlist')
      await playlistCard.trigger('click')

      const playlistTracks = wrapper
        .get('[aria-label="沉浸模式本地歌单"]')
        .findAll<HTMLButtonElement>('.music-immersive-playlist-row.compact')
      expect(playlistTracks).toHaveLength(2)
      await playlistTracks[1].trigger('click')
      await flushPromises()

      const nextTrackLyricsCall = tauriCommandMock.mock.calls.find(
        ([command, args]) =>
          command === 'read_music_lyrics' &&
          (args as { path?: string } | undefined)?.path === 'test-data/next.mp3',
      )
      expect(nextTrackLyricsCall).toBeDefined()

      await vi.waitFor(
        () => {
          const lyricStage = wrapper.get('[aria-label="沉浸歌词"]')
          expect(lyricStage.classes()).not.toContain('is-loading')
          expect(lyricStage.get('.music-immersive-lyrics-line.current').text()).toBe('切歌歌词已加载')
        },
        { timeout: 500 },
      )
      expect(wrapper.get('.music-window').classes()).toContain(`theme-${theme}`)

      wrapper.unmount()
      await flushPromises()
    },
  )
})
