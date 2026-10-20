import manifest from './manifest.json' with { type: 'json' }

export const dictionaries = manifest.dictionaries.map(({ id, upstream }) => ({
  id,
  upstream: `${upstream.package}@${upstream.version}`,
  license: upstream.license,
}))

export const dictionaryIds = dictionaries.map((dictionary) => dictionary.id)
