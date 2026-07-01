import { invoke } from '@tauri-apps/api/core'
import { ref, shallowRef } from 'vue'

export interface MusicLyricsTrack {
  id: string
  path: string
  sourcePath?: string
  title: string
  artist: string
  duration: number | null
  source?: 'local' | 'netease' | 'kugou'
  neteaseSongId?: number
  kugouSongHash?: string
}

export interface MusicLyricLine {
  time: number | null
  text: string
  duration?: number | null
  words?: MusicLyricWord[]
  source?: 'lrc' | 'plain' | 'yrc-line' | 'yrc-word'
}

export interface MusicLyricWord {
  text: string
  time: number
  duration: number
  startChar: number
  endChar: number
}

export interface MusicLyricsWindow {
  previous: string
  current: string
  next: string
  previousKey: string
  currentKey: string
  nextKey: string
  progress: number
  interlude: boolean
  karaoke: boolean
  status: MusicLyricsStatus
  synced: boolean
}

export type MusicLyricsStatus = 'idle' | 'loading' | 'ready' | 'empty' | 'error'

interface MusicLyricsResult {
  content: string
  lrcContent?: string | null
  yrcContent?: string | null
  klyricContent?: string | null
  translatedContent?: string | null
  source: string
  warnings: string[]
}

const LINE_SWITCH_EARLY_TOLERANCE_SECONDS = 0.18
const LONG_INTERLUDE_GAP_SECONDS = 8
const INTERLUDE_FADE_AFTER_SECONDS = 0.65
const DEFAULT_LAST_LINE_SWEEP_SECONDS = 4.2

export function useMusicLyrics() {
  const lyricsStatus = ref<MusicLyricsStatus>('idle')
  const lyricsError = ref('')
  const lyricsSource = ref('')
  const lyricLines = shallowRef<MusicLyricLine[]>([])
  const activeTrackId = ref('')

  let requestId = 0

  async function loadLyricsForTrack(track: MusicLyricsTrack | null) {
    requestId += 1
    const currentRequestId = requestId
    lyricsError.value = ''
    lyricsSource.value = ''
    lyricLines.value = []
    activeTrackId.value = track?.id ?? ''

    if (!track) {
      lyricsStatus.value = 'idle'
      return
    }

    lyricsStatus.value = 'loading'

    try {
      const result =
        track.source === 'netease' && track.neteaseSongId
          ? await invoke<MusicLyricsResult | null>('read_netease_lyrics', {
              songId: track.neteaseSongId,
            })
          : track.source === 'kugou' && track.kugouSongHash
            ? await invoke<MusicLyricsResult | null>('read_kugou_lyrics', {
                hash: track.kugouSongHash,
                name: track.title,
                artist: track.artist,
                durationMs: track.duration ? Math.round(track.duration * 1000) : null,
              })
          : await invoke<MusicLyricsResult | null>('read_music_lyrics', {
              path: track.path,
              sourcePath: track.sourcePath ?? track.path,
            })
      if (currentRequestId !== requestId) {
        return
      }

      const parsedLyrics = parseLyricsResult(result)
      if (parsedLyrics.lines.length === 0) {
        lyricsStatus.value = 'empty'
        return
      }

      lyricLines.value = parsedLyrics.lines
      lyricsSource.value = parsedLyrics.source || result?.source || ''
      lyricsStatus.value = lyricLines.value.length > 0 ? 'ready' : 'empty'
      lyricsError.value = result?.warnings.join('；') ?? ''
    } catch (err) {
      if (currentRequestId !== requestId) {
        return
      }

      lyricsStatus.value = 'error'
      lyricsError.value = `歌词读取失败：${String(err)}`
    }
  }

  function resetLyrics() {
    requestId += 1
    activeTrackId.value = ''
    lyricLines.value = []
    lyricsStatus.value = 'idle'
    lyricsError.value = ''
    lyricsSource.value = ''
  }

  function lyricsAt(time: number, track: MusicLyricsTrack | null): MusicLyricsWindow {
    const safeTime = Number.isFinite(time) ? Math.max(0, time) : 0

    if (!track || activeTrackId.value !== track.id) {
      return fallbackLyrics(track, lyricsStatus.value, false)
    }

    if (lyricsStatus.value !== 'ready' || lyricLines.value.length === 0) {
      return fallbackLyrics(track, lyricsStatus.value, false)
    }

    const synced = lyricLines.value.some((line) => line.time !== null)
    const index = synced
      ? syncedLineIndex(lyricLines.value, safeTime)
      : unsyncedLineIndex(lyricLines.value, safeTime, track.duration)
    const line = lyricLines.value[index]
    const progress = synced
      ? syncedLineProgress(lyricLines.value, index, safeTime, track.duration)
      : unsyncedLineProgress(lyricLines.value, index, safeTime, track.duration)
    const interlude = synced
      ? syncedLineInterlude(lyricLines.value, index, safeTime, track.duration)
      : false
    const karaoke = Boolean(line?.words?.length)

    return {
      previous: lyricLines.value[index - 1]?.text ?? '',
      current: line?.text || track.title,
      next: lyricLines.value[index + 1]?.text ?? '',
      previousKey: lyricWindowKey(track.id, index - 1, lyricLines.value[index - 1]?.text ?? ''),
      currentKey: lyricWindowKey(track.id, index, line?.text || track.title),
      nextKey: lyricWindowKey(track.id, index + 1, lyricLines.value[index + 1]?.text ?? ''),
      progress,
      interlude,
      karaoke,
      status: lyricsStatus.value,
      synced,
    }
  }

  return {
    lyricsStatus,
    lyricsError,
    lyricsSource,
    lyricLines,
    loadLyricsForTrack,
    resetLyrics,
    lyricsAt,
  }
}

function parseLyricsResult(result: MusicLyricsResult | null | undefined) {
  const yrcLines = parseYrcLyrics(result?.yrcContent ?? '')
  if (yrcLines.length > 0) {
    return {
      lines: yrcLines,
      source: yrcLines.some((line) => line.words?.length) ? 'netease-yrc-word' : 'netease-yrc-line',
    }
  }

  const lrcContent = result?.lrcContent?.trim() || result?.content?.trim() || ''
  const lrcLines = parseLyrics(lrcContent)
  if (lrcLines.length > 0) {
    return {
      lines: lrcLines,
      source: result?.source ?? '',
    }
  }

  const fallbackContent =
    result?.klyricContent?.trim() || result?.translatedContent?.trim() || ''
  return {
    lines: parseLyrics(fallbackContent),
    source: result?.source ?? '',
  }
}

function parseLyrics(content: string): MusicLyricLine[] {
  const normalized = content.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  const parsedLines: MusicLyricLine[] = []
  const plainLines: MusicLyricLine[] = []

  for (const rawLine of normalized.split('\n')) {
    const line = rawLine.trim()
    if (!line) {
      continue
    }

    const timestamps = [...line.matchAll(/\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]/g)]
    if (timestamps.length === 0) {
      if (!isLrcMetadataLine(line)) {
        plainLines.push({ time: null, text: stripLrcTags(line), source: 'plain' })
      }
      continue
    }

    const text = line.replace(/\[(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?\]/g, '').trim()
    if (!text || isLrcMetadataLine(line)) {
      continue
    }

    for (const timestamp of timestamps) {
      const time = timestampToSeconds(timestamp[1], timestamp[2], timestamp[3])
      parsedLines.push({ time, text, source: 'lrc' })
    }
  }

  if (parsedLines.length > 0) {
    return finalizeTimedLyricLines(parsedLines)
  }

  return plainLines
    .map((line) => ({ time: null, text: line.text.trim() }))
    .filter((line) => line.text)
}

function parseYrcLyrics(content: string): MusicLyricLine[] {
  const lines: MusicLyricLine[] = []

  for (const rawLine of content.replace(/\r\n/g, '\n').replace(/\r/g, '\n').split('\n')) {
    const line = rawLine.trim()
    const match = line.match(/^\[(\d+),(\d+)\](.*)$/)
    if (!match) {
      continue
    }

    const lineStartMs = Number(match[1] ?? 0)
    const lineDurationMs = Number(match[2] ?? 0)
    const body = match[3] ?? ''
    if (!Number.isFinite(lineStartMs)) {
      continue
    }

    const words: MusicLyricWord[] = []
    let fullText = ''
    const wordPattern = /\((\d+),(\d+),\d+\)([^()]*)/g
    let wordMatch: RegExpExecArray | null = null

    while ((wordMatch = wordPattern.exec(body))) {
      const text = (wordMatch[3] ?? '').replace(/\s+/g, ' ')
      if (!text) {
        continue
      }

      const rawStartMs = Number(wordMatch[1] ?? 0)
      const rawDurationMs = Number(wordMatch[2] ?? 0)
      const absoluteStartMs =
        rawStartMs >= lineStartMs - 500 ? rawStartMs : lineStartMs + rawStartMs
      const startChar = fullText.length
      fullText += text
      words.push({
        text,
        time: absoluteStartMs / 1000,
        duration: Math.max(0.06, rawDurationMs / 1000),
        startChar,
        endChar: fullText.length,
      })
    }

    if (!fullText) {
      fullText = body.replace(/\(\d+,\d+,\d+\)/g, '').replace(/\s+/g, ' ')
    }

    const leadingWhitespace = fullText.match(/^\s+/)?.[0].length ?? 0
    fullText = fullText.replace(/\s+/g, ' ').trim()
    if (!fullText || isNoLyricText(fullText)) {
      continue
    }

    const normalizedWords = words
      .map((word) => ({
        ...word,
        startChar: clampIndex(word.startChar - leadingWhitespace, fullText.length),
        endChar: clampIndex(word.endChar - leadingWhitespace, fullText.length),
      }))
      .filter((word) => word.endChar > word.startChar)

    lines.push({
      time: lineStartMs / 1000,
      duration: Math.max(0.45, lineDurationMs / 1000),
      text: fullText,
      words: normalizedWords.length > 0 ? normalizedWords : undefined,
      source: normalizedWords.length > 0 ? 'yrc-word' : 'yrc-line',
    })
  }

  return finalizeTimedLyricLines(lines)
}

function finalizeTimedLyricLines(lines: MusicLyricLine[]) {
  const sorted = lines
    .filter((line) => line.time !== null && line.time !== undefined && line.text.trim())
    .sort((left, right) => (left.time ?? 0) - (right.time ?? 0))

  for (let index = 0; index < sorted.length; index += 1) {
    const line = sorted[index]
    const currentTime = line.time ?? 0
    const nextTime = sorted[index + 1]?.time
    const inferredDuration =
      nextTime !== null && nextTime !== undefined && nextTime > currentTime
        ? nextTime - currentTime
        : DEFAULT_LAST_LINE_SWEEP_SECONDS
    line.duration = clamp(line.duration ?? inferredDuration, 0.45, 12)
  }

  return sorted
}

function syncedLineIndex(lines: MusicLyricLine[], time: number) {
  let activeIndex = 0
  const safeTime = Number.isFinite(time) ? time : 0

  for (let index = 0; index < lines.length; index += 1) {
    const lineTime = lines[index]?.time
    if (
      lineTime === null ||
      lineTime === undefined ||
      lineTime > safeTime + LINE_SWITCH_EARLY_TOLERANCE_SECONDS
    ) {
      break
    }
    activeIndex = index
  }

  return activeIndex
}

function unsyncedLineIndex(lines: MusicLyricLine[], time: number, duration: number | null) {
  if (!duration || duration <= 0 || lines.length <= 1) {
    return 0
  }

  const progress = clamp(time / duration, 0, 0.999)
  return Math.min(lines.length - 1, Math.floor(progress * lines.length))
}

function syncedLineProgress(
  lines: MusicLyricLine[],
  index: number,
  time: number,
  duration: number | null,
) {
  const line = lines[index]
  if (line?.words?.length) {
    return yrcWordLineProgress(line, time)
  }

  const currentTime = lines[index]?.time
  if (currentTime === null || currentTime === undefined) {
    return 0
  }

  const nextTime = nextTimedLineTime(lines, index)
  const endTime = lyricLineSweepEndTime(lines[index]?.text ?? '', currentTime, nextTime, duration)
  return clamp((time - currentTime) / Math.max(0.8, endTime - currentTime), 0, 1)
}

function yrcWordLineProgress(line: MusicLyricLine, time: number) {
  const words = line.words ?? []
  const charCount = Math.max(1, line.text.length)
  const adjustedTime = time + 0.03
  let lastProgress = 0

  for (const word of words) {
    const wordStart = word.time
    const wordEnd = word.time + Math.max(0.08, word.duration || 0.24)
    if (adjustedTime < wordStart) {
      return lastProgress
    }

    const localProgress =
      adjustedTime >= wordEnd
        ? 1
        : (adjustedTime - wordStart) / Math.max(0.08, wordEnd - wordStart)
    const progress =
      (word.startChar + (word.endChar - word.startChar) * clamp(localProgress, 0, 1)) /
      charCount
    lastProgress = Math.max(lastProgress, progress)

    if (adjustedTime < wordEnd) {
      return lastProgress
    }
  }

  return words.length > 0 ? 1 : 0
}

function syncedLineInterlude(
  lines: MusicLyricLine[],
  index: number,
  time: number,
  duration: number | null,
) {
  const currentTime = lines[index]?.time
  if (currentTime === null || currentTime === undefined) {
    return false
  }

  const nextTime = nextTimedLineTime(lines, index)
  const rawEndTime =
    nextTime ?? (duration && duration > currentTime ? duration : currentTime + DEFAULT_LAST_LINE_SWEEP_SECONDS)
  const gap = rawEndTime - currentTime
  if (gap <= LONG_INTERLUDE_GAP_SECONDS) {
    return false
  }

  const sweepEndTime = lyricLineSweepEndTime(lines[index]?.text ?? '', currentTime, nextTime, duration)
  return time > sweepEndTime + INTERLUDE_FADE_AFTER_SECONDS
}

function unsyncedLineProgress(
  lines: MusicLyricLine[],
  index: number,
  time: number,
  duration: number | null,
) {
  if (!duration || duration <= 0 || lines.length <= 1) {
    return 0
  }

  const lineDuration = duration / lines.length
  return clamp((time - index * lineDuration) / Math.max(0.8, lineDuration), 0, 1)
}

function nextTimedLineTime(lines: MusicLyricLine[], index: number) {
  for (let nextIndex = index + 1; nextIndex < lines.length; nextIndex += 1) {
    const lineTime = lines[nextIndex]?.time
    if (lineTime !== null && lineTime !== undefined) {
      return lineTime
    }
  }

  return null
}

function lyricLineSweepEndTime(
  text: string,
  currentTime: number,
  nextTime: number | null,
  duration: number | null,
) {
  const fallbackEnd = duration && duration > currentTime
    ? duration
    : currentTime + DEFAULT_LAST_LINE_SWEEP_SECONDS
  const rawEndTime = nextTime ?? fallbackEnd
  const rawGap = rawEndTime - currentTime

  if (rawGap <= LONG_INTERLUDE_GAP_SECONDS) {
    return rawEndTime
  }

  return Math.min(rawEndTime, currentTime + estimateLyricLineVocalSeconds(text))
}

function estimateLyricLineVocalSeconds(text: string) {
  const compactText = text.trim()
  if (!compactText) {
    return DEFAULT_LAST_LINE_SWEEP_SECONDS
  }

  const cjkMatches = compactText.match(/[\u3400-\u9fff\uf900-\ufaff]/g) ?? []
  const latinWords = compactText
    .replace(/[\u3400-\u9fff\uf900-\ufaff]/g, ' ')
    .split(/\s+/)
    .filter(Boolean)
  const units = Math.max(1, cjkMatches.length + latinWords.length)
  return clamp(1.8 + units * 0.18, 2.4, 5.8)
}

function fallbackLyrics(
  track: MusicLyricsTrack | null,
  status: MusicLyricsStatus,
  synced: boolean,
): MusicLyricsWindow {
  if (!track) {
    return {
      previous: '',
      current: '选择音乐',
      next: '沉浸歌词会显示在这里',
      previousKey: 'empty-previous',
      currentKey: 'empty-current',
      nextKey: 'empty-next',
      progress: 0,
      interlude: false,
      karaoke: false,
      status,
      synced,
    }
  }

  const current = track.title
  const onlineTrack = track.source === 'netease' || track.source === 'kugou'
  const previous = status === 'loading' ? (onlineTrack ? '正在读取在线歌词' : '正在读取本机歌词') : ''
  const next = track.artist || (onlineTrack ? '未找到在线歌词' : '未找到同名歌词文件')

  return {
    previous,
    current,
    next,
    previousKey: lyricWindowKey(track.id, -1, previous),
    currentKey: lyricWindowKey(track.id, 0, current),
    nextKey: lyricWindowKey(track.id, 1, next),
    progress: 0,
    interlude: false,
    karaoke: false,
    status,
    synced,
  }
}

function lyricWindowKey(trackId: string, index: number, text: string) {
  return `${trackId}:${index}:${text}`
}

function timestampToSeconds(minutes?: string, seconds?: string, fraction?: string) {
  const minuteValue = Number(minutes ?? 0)
  const secondValue = Number(seconds ?? 0)
  const fractionValue = Number((fraction ?? '').padEnd(3, '0').slice(0, 3))
  return minuteValue * 60 + secondValue + fractionValue / 1000
}

function isLrcMetadataLine(line: string) {
  return /^\[(ti|ar|al|by|offset|length|re):/i.test(line)
}

function isNoLyricText(text: string) {
  const compact = text.replace(/\s+/g, '').replace(/[，,。.!！?？、~～]/g, '')
  return (
    !compact ||
    compact === '纯音乐请欣赏' ||
    compact === '暂无歌词' ||
    compact === '暂无歌词敬请期待' ||
    compact === '此歌曲为没有填词的纯音乐请您欣赏'
  )
}

function stripLrcTags(line: string) {
  return line.replace(/\[[^\]]+\]/g, '').trim()
}

function clampIndex(index: number, length: number) {
  if (!Number.isFinite(index) || length <= 0) {
    return 0
  }

  return Math.min(length, Math.max(0, Math.round(index)))
}

function clamp(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) {
    return min
  }

  return Math.min(max, Math.max(min, value))
}
