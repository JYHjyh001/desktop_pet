export function appNameFromPath(path: string): string {
  const normalized = path.replace(/\\/g, '/')
  const fileName = normalized.split('/').pop() ?? ''
  return fileName.replace(/\.[^.]+$/, '') || '新软件'
}

export function folderNameFromPath(path: string): string {
  const normalized = path.replace(/\\/g, '/').replace(/\/+$/, '')
  return normalized.split('/').pop() || normalized || '新文件夹'
}

export function fileNameFromPath(path: string): string {
  const normalized = path.replace(/\\/g, '/')
  return normalized.split('/').pop() || '新文件'
}

export function websiteNameFromUrl(url: string): string {
  try {
    const parsed = new URL(normalizeWebsiteUrl(url))
    return parsed.hostname.replace(/^www\./, '') || '新网站'
  } catch {
    return '新网站'
  }
}

export function normalizeWebsiteUrl(url: string): string {
  const trimmed = url.trim()
  if (!trimmed) {
    return ''
  }

  if (/^https?:\/\//i.test(trimmed)) {
    return trimmed
  }

  return `https://${trimmed}`
}

export function parseTags(value: string): string[] {
  return value
    .split(/[,\s，、]+/)
    .map((tag) => tag.trim())
    .filter(Boolean)
}

export function formatLaunchTime(value?: string | null): string {
  if (!value) {
    return '未启动'
  }

  const seconds = Number(value)
  if (!Number.isFinite(seconds)) {
    return value
  }

  return new Date(seconds * 1000).toLocaleString()
}
