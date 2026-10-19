// Public, read-only verifier. Standalone Node.js; no signing key, npm install or app build.
import { createHash, createPublicKey, verify } from 'node:crypto'
import { lstat, readFile } from 'node:fs/promises'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

const digest = bytes => createHash('sha256').update(bytes).digest('hex')
export const packageIds = ['official.music', 'official.music-stage']
export const officialTrust = Object.freeze({
  formatVersion: 1,
  keyId: 'petdrawer-official-20261019',
  publicKey: 'How5w249oJ9inuJWGrA9Onej84qqDLJ/LdHhvVc40yU=',
  allowedOrigins: Object.freeze(['https://raw.githubusercontent.com']),
  catalogUrl: 'https://raw.githubusercontent.com/JYHjyh001/desktop_pet/refs/heads/codex/feature-packages/catalog.json',
})
export function validatePublication(config, now = Math.floor(Date.now() / 1000)) {
  if (config.formatVersion !== 1 || !/^[A-Za-z0-9._-]{1,80}$/.test(config.keyId) ||
      !Number.isSafeInteger(config.sequence) || config.sequence < 1 ||
      !Number.isSafeInteger(config.expiresAt) || config.expiresAt <= now || config.expiresAt > now + 30 * 86400 ||
      !Array.isArray(config.allowedOrigins) || !config.allowedOrigins.length || config.allowedOrigins.length > 8 ||
      typeof config.publicKey !== 'string' || Buffer.from(config.publicKey, 'base64').length !== 32) throw new Error('Invalid publication configuration')
  const allowed = new Set(config.allowedOrigins)
  for (const value of allowed) {
    const url = new URL(value)
    if (url.protocol !== 'https:' || url.origin !== value) throw new Error('Exact HTTPS origins required')
  }
  const urls = [config.catalogUrl, ...packageIds.map(id => config.packageUrls?.[id])]
  for (const value of urls) {
    if (typeof value !== 'string') throw new Error('Publication URL missing')
    const url = new URL(value)
    if (url.protocol !== 'https:' || !allowed.has(url.origin) || url.username || url.password || url.hash || url.search || url.pathname.endsWith('/')) throw new Error('Approved direct HTTPS URLs without credentials required')
  }
  if (new Set(urls).size !== urls.length) throw new Error('Publication URLs must be distinct')
}
function base64(value, limit) {
  if (typeof value !== 'string' || value.length > Math.ceil(limit / 3) * 4) throw new Error('Invalid encoded document')
  const bytes = Buffer.from(value, 'base64')
  if (bytes.length > limit || bytes.toString('base64') !== value) throw new Error('Invalid encoded document')
  return bytes
}
export function verifyPublication(artifacts, trust, now = Math.floor(Date.now() / 1000)) {
  const publicBytes = base64(trust.publicKey, 32)
  if (publicBytes.length !== 32) throw new Error('Ed25519 public key required')
  const key = createPublicKey({ key: { kty: 'OKP', crv: 'Ed25519', x: publicBytes.toString('base64url') }, format: 'jwk' })
  const unpack = (kind, bytes) => {
    if (!Buffer.isBuffer(bytes) || bytes.length > 2 * 1024 * 1024) throw new Error('Invalid signed document')
    const signed = JSON.parse(bytes)
    const raw = base64(signed.payload, 1024 * 1024)
    const signature = base64(signed.signature, 64)
    if (signed.keyId !== trust.keyId || signature.length !== 64 || !verify(null, Buffer.concat([Buffer.from(`PetDrawer:${kind}:v1\0`), raw]), key, signature)) throw new Error('Publication signature rejected')
    return { value: JSON.parse(raw), raw }
  }
  const catalog = unpack('catalog', artifacts.get('catalog.json')).value
  if (catalog.formatVersion !== 1 || !Array.isArray(catalog.packages) || catalog.packages.length !== 2 ||
      new Set(catalog.packages.map(e => e.id)).size !== 2 || catalog.packages.some(e => !packageIds.includes(e.id))) throw new Error('Two distinct official packages required')
  validatePublication({ ...trust, sequence: catalog.sequence, expiresAt: catalog.expiresAt, packageUrls: Object.fromEntries(catalog.packages.map(e => [e.id, e.url])) }, now)
  for (const entry of catalog.packages) {
    if (!/^\d+\.\d+\.\d+$/.test(entry.version) || !Number.isSafeInteger(entry.releaseSequence) || entry.releaseSequence < 1 ||
        !Number.isSafeInteger(entry.size) || entry.size < 1 || entry.size > 64 * 1024 * 1024 ||
        !/^[a-f0-9]{64}$/.test(entry.sha256) || !/^[a-f0-9]{64}$/.test(entry.manifestSha256)) throw new Error('Invalid catalog entry')
    const file = `${entry.id}-${entry.version}-${entry.releaseSequence}.petpack`
    if (entry.url !== new URL(file, trust.catalogUrl).href) throw new Error('Immutable filename and download address mismatch')
    const bytes = artifacts.get(file)
    if (!Buffer.isBuffer(bytes) || bytes.length !== entry.size || digest(bytes) !== entry.sha256) throw new Error('Publication bundle digest rejected')
    const bundle = JSON.parse(bytes)
    const { value: manifest, raw } = unpack('manifest', Buffer.from(JSON.stringify(bundle.manifest)))
    if (bundle.formatVersion !== 1 || manifest.formatVersion !== 1 || digest(raw) !== entry.manifestSha256 ||
        ['id', 'name', 'version', 'releaseSequence'].some(k => manifest[k] !== entry[k])) throw new Error('Publication identity mismatch')
    if (!Array.isArray(bundle.files) || !Array.isArray(manifest.files) || !manifest.files.length || manifest.files.length > 4096 || bundle.files.length !== manifest.files.length) throw new Error('Publication file inventory rejected')
    const payload = new Map()
    let total = 0
    for (const record of manifest.files) {
      if (typeof record.path !== 'string' || !record.path || /[\\:\x00-\x1f]/.test(record.path) ||
          record.path.split('/').some(s => !s || s === '.' || s === '..') ||
          !Number.isSafeInteger(record.size) || record.size < 0 || record.size > 32 * 1024 * 1024 ||
          !/^[a-f0-9]{64}$/.test(record.sha256) || payload.has(record.path.toLowerCase())) throw new Error('Publication file inventory rejected')
      total += record.size
      if (total > 48 * 1024 * 1024) throw new Error('Publication file inventory rejected')
      payload.set(record.path.toLowerCase(), record)
    }
    const seen = new Set()
    for (const file of bundle.files) {
      const record = typeof file.path === 'string' ? payload.get(file.path.toLowerCase()) : undefined
      if (!record || record.path !== file.path || seen.has(file.path)) throw new Error('Publication file inventory rejected')
      const content = base64(file.content, record.size)
      if (content.length !== record.size || digest(content) !== record.sha256) throw new Error('Publication file inventory rejected')
      seen.add(file.path)
    }
  }
  return catalog
}
export function verifyDistribution(artifacts, now = Math.floor(Date.now() / 1000), approved = officialTrust) {
  const trust = JSON.parse(artifacts.get('trust-config.json'))
  if (Object.keys(trust).sort().join(',') !== Object.keys(approved).sort().join(',') ||
      ['formatVersion', 'keyId', 'publicKey', 'catalogUrl'].some(k => trust[k] !== approved[k]) ||
      JSON.stringify(trust.allowedOrigins) !== JSON.stringify(approved.allowedOrigins)) throw new Error('Distribution trust differs from reviewed public identity')
  return verifyPublication(artifacts, approved, now)
}
async function main(args) {
  if (args.length > 1) throw new Error('Use one local distribution directory')
  const root = path.resolve(args[0] || '.')
  const artifacts = new Map()
  const readArtifact = async file => {
    const target = path.join(root, file)
    const info = await lstat(target)
    const limit = file.endsWith('.petpack') ? 64 * 1024 * 1024 : file === 'catalog.json' ? 2 * 1024 * 1024 : 16384
    if (!info.isFile() || info.isSymbolicLink() || info.size > limit) throw new Error('Invalid distribution file')
    artifacts.set(file, await readFile(target))
  }
  for (const file of ['catalog.json', 'trust-config.json']) await readArtifact(file)
  // These preliminary names only select bounded local reads. Trust/signatures are
  // verified below before success; archived immutable versions need not be deleted.
  const envelope = JSON.parse(artifacts.get('catalog.json'))
  const candidate = JSON.parse(base64(envelope.payload, 1024 * 1024))
  if (!Array.isArray(candidate.packages) || candidate.packages.length !== 2) throw new Error('Two declared packages required')
  for (const entry of candidate.packages) {
    if (!packageIds.includes(entry.id) || !/^\d+\.\d+\.\d+$/.test(entry.version) || !Number.isSafeInteger(entry.releaseSequence) || entry.releaseSequence < 1) throw new Error('Invalid package identity')
    await readArtifact(`${entry.id}-${entry.version}-${entry.releaseSequence}.petpack`)
  }
  const catalog = verifyDistribution(artifacts)
  console.log(`Distribution verified: catalog sequence ${catalog.sequence}; ${catalog.packages.length} packages; signatures, digests, inventory and exact download addresses match. No files changed or executed.`)
}
if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch(() => { console.error('Distribution verification failed: check reviewed trust, signatures, expiry, digests, inventory and addresses.'); process.exitCode = 1 })
}
