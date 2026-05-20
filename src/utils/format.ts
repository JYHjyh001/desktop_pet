export function appNameFromPath(path: string): string {
  const normalized = path.replace(/\\/g, '/')
  const fileName = normalized.split('/').pop() ?? ''
  return fileName.replace(/\.[^.]+$/, '') || '新软件'
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
