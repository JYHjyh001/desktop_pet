export type TwemojiItem = {
  code: string
  emoji: string
  label: string
  src: string
}

export type TwemojiTextToken =
  | {
      type: 'text'
      text: string
    }
  | {
      type: 'emoji'
      text: string
      label: string
      src: string
    }

const twemojiAssetSrc: Record<string, string> = {
  '1f600': new URL('../assets/emoji/twemoji/svg/1f600.svg', import.meta.url).href,
  '1f604': new URL('../assets/emoji/twemoji/svg/1f604.svg', import.meta.url).href,
  '1f606': new URL('../assets/emoji/twemoji/svg/1f606.svg', import.meta.url).href,
  '1f602': new URL('../assets/emoji/twemoji/svg/1f602.svg', import.meta.url).href,
  '1f605': new URL('../assets/emoji/twemoji/svg/1f605.svg', import.meta.url).href,
  '1f609': new URL('../assets/emoji/twemoji/svg/1f609.svg', import.meta.url).href,
  '1f60a': new URL('../assets/emoji/twemoji/svg/1f60a.svg', import.meta.url).href,
  '1f60d': new URL('../assets/emoji/twemoji/svg/1f60d.svg', import.meta.url).href,
  '1f970': new URL('../assets/emoji/twemoji/svg/1f970.svg', import.meta.url).href,
  '1f917': new URL('../assets/emoji/twemoji/svg/1f917.svg', import.meta.url).href,
  '1f914': new URL('../assets/emoji/twemoji/svg/1f914.svg', import.meta.url).href,
  '1f60e': new URL('../assets/emoji/twemoji/svg/1f60e.svg', import.meta.url).href,
  '1f973': new URL('../assets/emoji/twemoji/svg/1f973.svg', import.meta.url).href,
  '1f62d': new URL('../assets/emoji/twemoji/svg/1f62d.svg', import.meta.url).href,
  '1f621': new URL('../assets/emoji/twemoji/svg/1f621.svg', import.meta.url).href,
  '1f634': new URL('../assets/emoji/twemoji/svg/1f634.svg', import.meta.url).href,
  '1f44d': new URL('../assets/emoji/twemoji/svg/1f44d.svg', import.meta.url).href,
  '1f44f': new URL('../assets/emoji/twemoji/svg/1f44f.svg', import.meta.url).href,
  '1f64f': new URL('../assets/emoji/twemoji/svg/1f64f.svg', import.meta.url).href,
  '1f4aa': new URL('../assets/emoji/twemoji/svg/1f4aa.svg', import.meta.url).href,
  '1f440': new URL('../assets/emoji/twemoji/svg/1f440.svg', import.meta.url).href,
  '2728': new URL('../assets/emoji/twemoji/svg/2728.svg', import.meta.url).href,
  '2764': new URL('../assets/emoji/twemoji/svg/2764.svg', import.meta.url).href,
  '1f525': new URL('../assets/emoji/twemoji/svg/1f525.svg', import.meta.url).href,
  '2b50': new URL('../assets/emoji/twemoji/svg/2b50.svg', import.meta.url).href,
  '1f389': new URL('../assets/emoji/twemoji/svg/1f389.svg', import.meta.url).href,
  '1f381': new URL('../assets/emoji/twemoji/svg/1f381.svg', import.meta.url).href,
  '1f31f': new URL('../assets/emoji/twemoji/svg/1f31f.svg', import.meta.url).href,
  '1f496': new URL('../assets/emoji/twemoji/svg/1f496.svg', import.meta.url).href,
  '1f4ac': new URL('../assets/emoji/twemoji/svg/1f4ac.svg', import.meta.url).href,
  '2705': new URL('../assets/emoji/twemoji/svg/2705.svg', import.meta.url).href,
  '2753': new URL('../assets/emoji/twemoji/svg/2753.svg', import.meta.url).href,
  '1f43e': new URL('../assets/emoji/twemoji/svg/1f43e.svg', import.meta.url).href,
  '1f431': new URL('../assets/emoji/twemoji/svg/1f431.svg', import.meta.url).href,
  '1f436': new URL('../assets/emoji/twemoji/svg/1f436.svg', import.meta.url).href,
}

const twemojiSrc = (code: string) => twemojiAssetSrc[code]

export const twemojiItems: TwemojiItem[] = [
  { code: '1f600', emoji: '😀', label: '笑脸', src: twemojiSrc('1f600') },
  { code: '1f604', emoji: '😄', label: '开心', src: twemojiSrc('1f604') },
  { code: '1f606', emoji: '😆', label: '大笑', src: twemojiSrc('1f606') },
  { code: '1f602', emoji: '😂', label: '笑哭', src: twemojiSrc('1f602') },
  { code: '1f605', emoji: '😅', label: '松一口气', src: twemojiSrc('1f605') },
  { code: '1f609', emoji: '😉', label: '眨眼', src: twemojiSrc('1f609') },
  { code: '1f60a', emoji: '😊', label: '微笑', src: twemojiSrc('1f60a') },
  { code: '1f60d', emoji: '😍', label: '喜欢', src: twemojiSrc('1f60d') },
  { code: '1f970', emoji: '🥰', label: '暖心', src: twemojiSrc('1f970') },
  { code: '1f917', emoji: '🤗', label: '拥抱', src: twemojiSrc('1f917') },
  { code: '1f914', emoji: '🤔', label: '思考', src: twemojiSrc('1f914') },
  { code: '1f60e', emoji: '😎', label: '酷', src: twemojiSrc('1f60e') },
  { code: '1f973', emoji: '🥳', label: '庆祝', src: twemojiSrc('1f973') },
  { code: '1f62d', emoji: '😭', label: '大哭', src: twemojiSrc('1f62d') },
  { code: '1f621', emoji: '😡', label: '生气', src: twemojiSrc('1f621') },
  { code: '1f634', emoji: '😴', label: '困了', src: twemojiSrc('1f634') },
  { code: '1f44d', emoji: '👍', label: '赞', src: twemojiSrc('1f44d') },
  { code: '1f44f', emoji: '👏', label: '鼓掌', src: twemojiSrc('1f44f') },
  { code: '1f64f', emoji: '🙏', label: '拜托', src: twemojiSrc('1f64f') },
  { code: '1f4aa', emoji: '💪', label: '加油', src: twemojiSrc('1f4aa') },
  { code: '1f440', emoji: '👀', label: '看看', src: twemojiSrc('1f440') },
  { code: '2728', emoji: '✨', label: '闪光', src: twemojiSrc('2728') },
  { code: '2764', emoji: '❤️', label: '红心', src: twemojiSrc('2764') },
  { code: '1f525', emoji: '🔥', label: '火', src: twemojiSrc('1f525') },
  { code: '2b50', emoji: '⭐', label: '星星', src: twemojiSrc('2b50') },
  { code: '1f389', emoji: '🎉', label: '彩带', src: twemojiSrc('1f389') },
  { code: '1f381', emoji: '🎁', label: '礼物', src: twemojiSrc('1f381') },
  { code: '1f31f', emoji: '🌟', label: '亮星', src: twemojiSrc('1f31f') },
  { code: '1f496', emoji: '💖', label: '闪亮的心', src: twemojiSrc('1f496') },
  { code: '1f4ac', emoji: '💬', label: '对话', src: twemojiSrc('1f4ac') },
  { code: '2705', emoji: '✅', label: '完成', src: twemojiSrc('2705') },
  { code: '2753', emoji: '❓', label: '疑问', src: twemojiSrc('2753') },
  { code: '1f43e', emoji: '🐾', label: '爪印', src: twemojiSrc('1f43e') },
  { code: '1f431', emoji: '🐱', label: '猫脸', src: twemojiSrc('1f431') },
  { code: '1f436', emoji: '🐶', label: '狗脸', src: twemojiSrc('1f436') },
]

const twemojiMatches = new Map<string, TwemojiItem>()

for (const item of twemojiItems) {
  twemojiMatches.set(item.emoji, item)
}

const heartItem = twemojiItems.find((item) => item.code === '2764')
if (heartItem) {
  twemojiMatches.set('❤', heartItem)
}

const twemojiMatchEntries = [...twemojiMatches.entries()].sort(
  ([left], [right]) => right.length - left.length,
)

export function tokenizeTwemojiText(text: string): TwemojiTextToken[] {
  const tokens: TwemojiTextToken[] = []
  let index = 0

  const appendText = (value: string) => {
    const previous = tokens[tokens.length - 1]
    if (previous?.type === 'text') {
      previous.text += value
      return
    }
    tokens.push({ type: 'text', text: value })
  }

  while (index < text.length) {
    const match = twemojiMatchEntries.find(([emoji]) => text.startsWith(emoji, index))
    if (match) {
      const [emoji, item] = match
      tokens.push({
        type: 'emoji',
        text: emoji,
        label: item.label,
        src: item.src,
      })
      index += emoji.length
      continue
    }

    const character = Array.from(text.slice(index))[0] ?? ''
    appendText(character)
    index += character.length
  }

  return tokens
}
