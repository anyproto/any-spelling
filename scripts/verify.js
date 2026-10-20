import { createHash } from 'node:crypto'
import { readFile, readdir, stat } from 'node:fs/promises'
import { join, relative } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { gunzipSync } from 'node:zlib'

const root = fileURLToPath(new URL('..', import.meta.url))
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex')

async function listFiles(dir) {
  const entries = await readdir(join(root, dir), { recursive: true, withFileTypes: true })
  return entries
    .filter((entry) => entry.isFile())
    .map((entry) => relative(root, join(entry.parentPath, entry.name)))
}

async function checkPinned(entries, errors) {
  for (const entry of entries) {
    const path = join(root, entry.file)
    const info = await stat(path).catch(() => null)
    if (info === null) {
      errors.push(`${entry.file}: missing`)
      continue
    }
    if (info.size !== entry.bytes) errors.push(`${entry.file}: ${info.size} bytes, manifest says ${entry.bytes}`)
    const hash = sha256(await readFile(path))
    if (hash !== entry.sha256) errors.push(`${entry.file}: sha256 ${hash}, manifest says ${entry.sha256}`)
  }
}

async function checkLocal(manifest) {
  const errors = []
  const files = manifest.dictionaries.flatMap((dictionary) => dictionary.files)
  await checkPinned(files, errors)
  await checkPinned(manifest.sources, errors)

  const pinned = new Set([...files, ...manifest.sources].map((entry) => entry.file))
  for (const file of [...(await listFiles('dictionaries')), ...(await listFiles('sources'))]) {
    if (!pinned.has(file)) errors.push(`${file}: not in manifest.json`)
  }

  const ids = manifest.dictionaries.map((dictionary) => dictionary.id)
  for (const dictionary of manifest.dictionaries) {
    for (const name of ['index.aff', 'index.dic', 'license']) {
      if (!pinned.has(`dictionaries/${dictionary.id}/${name}`)) errors.push(`${dictionary.id}: no ${name}`)
    }
  }
  for (const source of manifest.sources) {
    for (const id of source.languages) {
      if (!ids.includes(id)) errors.push(`${source.file}: unknown language ${id}`)
    }
  }

  const declared = [...(await readFile(join(root, 'index.d.ts'), 'utf8')).matchAll(/\|\s*'([^']+)'/g)].map((match) => match[1])
  if (declared.join() !== ids.join()) errors.push(`index.d.ts DictionaryId is [${declared}], manifest has [${ids}]`)

  return errors
}

function untar(buffer) {
  const files = new Map()
  for (let offset = 0; offset + 512 <= buffer.length; ) {
    const header = buffer.subarray(offset, offset + 512)
    if (header.every((byte) => byte === 0)) break
    const text = (start, end) => header.subarray(start, end).toString('utf8').replace(/\0.*$/s, '')
    const size = Number.parseInt(text(124, 136).trim(), 8)
    const prefix = text(345, 500)
    const name = prefix.length > 0 ? `${prefix}/${text(0, 100)}` : text(0, 100)
    if (text(156, 157) === '0' || text(156, 157) === '') {
      files.set(name, buffer.subarray(offset + 512, offset + 512 + size))
    }
    offset += 512 + Math.ceil(size / 512) * 512
  }
  return files
}

async function checkUpstream(manifest) {
  const errors = []
  for (const dictionary of manifest.dictionaries) {
    const response = await fetch(dictionary.upstream.tarball)
    if (!response.ok) {
      errors.push(`${dictionary.id}: ${dictionary.upstream.tarball} answered ${response.status}`)
      continue
    }
    const files = untar(gunzipSync(Buffer.from(await response.arrayBuffer())))
    for (const entry of dictionary.files) {
      const name = entry.file.split('/').at(-1)
      const bytes = files.get(`package/${name}`)
      if (bytes === undefined) errors.push(`${dictionary.id}: ${name} is absent from the npm tarball`)
      else if (sha256(bytes) !== entry.sha256) errors.push(`${dictionary.id}: ${name} differs from the npm tarball`)
    }
    console.log(`${dictionary.id}: checked against ${dictionary.upstream.package}@${dictionary.upstream.version}`)
  }
  return errors
}

async function main() {
  const manifest = JSON.parse(await readFile(join(root, 'manifest.json'), 'utf8'))
  const errors = await checkLocal(manifest)
  if (process.argv.includes('--upstream')) errors.push(...(await checkUpstream(manifest)))
  if (errors.length > 0) {
    for (const error of errors) console.error(error)
    process.exit(1)
  }
  const pinned = manifest.dictionaries.reduce((total, dictionary) => total + dictionary.files.length, 0)
  console.log(`ok: ${manifest.dictionaries.length} dictionaries, ${pinned} files, ${manifest.sources.length} source archives`)
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main()
