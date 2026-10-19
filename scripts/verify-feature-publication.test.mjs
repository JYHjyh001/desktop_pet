import test from 'node:test'
import assert from 'node:assert/strict'
import { createHash, generateKeyPairSync, sign } from 'node:crypto'
import { verifyDistribution, packageIds } from './verify-feature-publication.mjs'

const digest = bytes => createHash('sha256').update(bytes).digest('hex')
function fixture() {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519')
  const now = 1700000000
  const trust = { formatVersion: 1, keyId: 'synthetic-ci-only', publicKey: Buffer.from(publicKey.export({ format: 'jwk' }).x, 'base64url').toString('base64'), allowedOrigins: ['https://packages.invalid'], catalogUrl: 'https://packages.invalid/distribution/catalog.json' }
  const signed = (kind, value) => {
    const raw = Buffer.from(JSON.stringify(value))
    return { keyId: trust.keyId, payload: raw.toString('base64'), signature: sign(null, Buffer.concat([Buffer.from(`PetDrawer:${kind}:v1\0`), raw]), privateKey).toString('base64') }
  }
  const artifacts = new Map([['trust-config.json', Buffer.from(JSON.stringify(trust))]])
  const entries = packageIds.map(id => {
    const content = Buffer.from('<title>synthetic CI payload</title>')
    const manifest = { formatVersion: 1, id, name: id, version: '1.0.0', releaseSequence: 1, files: [{ path: 'index.html', size: content.length, sha256: digest(content) }] }
    const document = signed('manifest', manifest)
    const bytes = Buffer.from(JSON.stringify({ formatVersion: 1, manifest: document, files: [{ path: 'index.html', content: content.toString('base64') }] }))
    const file = `${id}-1.0.0-1.petpack`
    artifacts.set(file, bytes)
    return { id, name: id, version: '1.0.0', releaseSequence: 1, url: new URL(file, trust.catalogUrl).href, size: bytes.length, sha256: digest(bytes), manifestSha256: digest(Buffer.from(document.payload, 'base64')) }
  })
  const catalog = { formatVersion: 1, sequence: 1, expiresAt: now + 86400, packages: entries }
  const writeCatalog = () => artifacts.set('catalog.json', Buffer.from(JSON.stringify(signed('catalog', catalog))))
  writeCatalog()
  return { artifacts, trust, now, catalog, signed, writeCatalog }
}
test('distribution verifies both complete signed packages with an independently approved public identity', () => {
  const f = fixture()
  assert.equal(verifyDistribution(f.artifacts, f.now, f.trust).packages.length, 2)
  // The public CLI default never accepts the synthetic root from trust-config.json itself.
  assert.throws(() => verifyDistribution(f.artifacts, f.now), /trust/)
})
test('distribution rejects changed container bytes and a forged catalog signature', () => {
  const f = fixture()
  const file = 'official.music-1.0.0-1.petpack'
  const original = f.artifacts.get(file)
  f.artifacts.set(file, Buffer.concat([original, Buffer.from(' ')]))
  assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /digest/)
  f.artifacts.set(file, original)
  const catalog = JSON.parse(f.artifacts.get('catalog.json'))
  catalog.signature = Buffer.alloc(64).toString('base64')
  f.artifacts.set('catalog.json', Buffer.from(JSON.stringify(catalog)))
  assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /signature/)
})
test('distribution verifies nested manifest signatures and each declared file even after outer digest is updated', () => {
  for (const change of ['payload', 'manifest-signature', 'extra', 'duplicate', 'missing']) {
    const f = fixture(), file = 'official.music-1.0.0-1.petpack'
    const bundle = JSON.parse(f.artifacts.get(file))
    if (change === 'payload') bundle.files[0].content = Buffer.from('tampered').toString('base64')
    if (change === 'manifest-signature') bundle.manifest.signature = Buffer.alloc(64).toString('base64')
    if (change === 'extra') bundle.files.push({ path: 'extra.js', content: '' })
    if (change === 'duplicate') bundle.files.push(bundle.files[0])
    if (change === 'missing') bundle.files = []
    const bytes = Buffer.from(JSON.stringify(bundle))
    f.artifacts.set(file, bytes)
    Object.assign(f.catalog.packages[0], { size: bytes.length, sha256: digest(bytes) })
    f.writeCatalog()
    assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /inventory|signature/, change)
  }
})
test('distribution refuses another branch, mutable names, queries and missing declared packages', () => {
  for (const url of ['https://packages.invalid/other/official.music-1.0.0-1.petpack', 'https://packages.invalid/distribution/latest.petpack', 'https://packages.invalid/distribution/official.music-1.0.0-1.petpack?extra=1']) {
    const f = fixture()
    f.catalog.packages[0].url = url
    f.writeCatalog()
    assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /address|HTTPS/)
  }
  const f = fixture()
  f.artifacts.delete('official.music-1.0.0-1.petpack')
  assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /digest/)
})
test('distribution rejects replaced trust, expiry, catalog/container format and invalid inventory paths', () => {
  const f = fixture()
  const replaced = { ...f.trust, publicKey: fixture().trust.publicKey }
  f.artifacts.set('trust-config.json', Buffer.from(JSON.stringify(replaced)))
  assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /trust/)
  f.artifacts.set('trust-config.json', Buffer.from(JSON.stringify(f.trust)))
  assert.throws(() => verifyDistribution(f.artifacts, f.now + 86401, f.trust), /configuration/)
  f.catalog.formatVersion = 2; f.writeCatalog()
  assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /official packages/)
  for (const change of ['container', 'path']) {
    const f = fixture(), file = 'official.music-1.0.0-1.petpack'
    const bundle = JSON.parse(f.artifacts.get(file))
    if (change === 'container') bundle.formatVersion = 2
    else {
      const manifest = JSON.parse(Buffer.from(bundle.manifest.payload, 'base64'))
      manifest.files[0].path = '../outside'
      bundle.manifest = f.signed('manifest', manifest)
      bundle.files[0].path = '../outside'
      f.catalog.packages[0].manifestSha256 = digest(Buffer.from(bundle.manifest.payload, 'base64'))
    }
    const bytes = Buffer.from(JSON.stringify(bundle))
    f.artifacts.set(file, bytes)
    Object.assign(f.catalog.packages[0], { size: bytes.length, sha256: digest(bytes) })
    f.writeCatalog()
    assert.throws(() => verifyDistribution(f.artifacts, f.now, f.trust), /identity|inventory/)
  }
})
